use super::*;

use cmrt_tui_core::patch_load::PatchLoadState;

fn cfg_pointing_at_missing_patch_dir() -> Config {
    let missing = std::env::temp_dir().join("cmrt_test_http_patches_missing_dir_does_not_exist");
    std::fs::remove_dir_all(&missing).ok();
    Config {
        patches_dirs: Some(vec![missing.to_string_lossy().into_owned()]),
        ..Default::default()
    }
}

fn ready(pairs: &[&str]) -> Arc<Mutex<PatchLoadState>> {
    Arc::new(Mutex::new(PatchLoadState::ready(
        pairs
            .iter()
            .map(|patch| ((*patch).to_string(), patch.to_lowercase()))
            .collect(),
    )))
}

/// 実在しない patch dir を指す Config なら、走査経路は必ず 0 件。
/// それでも一覧が返るなら、走査していない（= snapshot 経路）ことの証拠になる。
#[test]
fn get_patches_uses_the_injected_snapshot_instead_of_scanning() {
    let cfg = cfg_pointing_at_missing_patch_dir();
    let patch_load = ready(&["Pads/Snapshot Pad.fxp", "Lead/Snapshot Lead.fxp"]);

    let started = std::time::Instant::now();
    let names = http_patch_names(&cfg, Some(&patch_load)).expect("snapshot 経路なら成功する");
    let elapsed = started.elapsed();

    assert_eq!(
        names,
        vec![
            "Pads/Snapshot Pad.fxp".to_string(),
            "Lead/Snapshot Lead.fxp".to_string()
        ]
    );
    // 走査経路は実測 1.3 秒。snapshot 経路なら桁違いに速い。
    assert!(
        elapsed < std::time::Duration::from_millis(100),
        "{elapsed:?}"
    );
}

/// snapshot が無いときは従来どおり走査へ落ちる（＝ 500 になる Config でも挙動が変わらない）。
#[test]
fn get_patches_falls_back_to_scanning_while_the_catalog_is_loading() {
    let cfg = cfg_pointing_at_missing_patch_dir();
    let patch_load = Arc::new(Mutex::new(PatchLoadState::Loading));

    let result = http_patch_names(&cfg, Some(&patch_load));

    let (status, message) = result.expect_err("実在しない patch dir の走査は Err になる");
    assert_eq!(status, 500);
    assert!(
        message.contains("patch 一覧の取得に失敗しました"),
        "{message}"
    );
}

/// `patches_dirs` が未設定で snapshot も無いときは、走査せず 0 件を返す
/// （走査するとこの開発機にインストール済みのプラグインの音色が数千件混ざる）。
#[test]
fn get_patches_returns_empty_without_scanning_when_patch_dirs_are_unset() {
    let cfg = Config::default();

    let started = std::time::Instant::now();
    let names = http_patch_names(&cfg, None).expect("未設定でも 200 で空一覧");
    let elapsed = started.elapsed();

    assert!(names.is_empty(), "{names:?}");
    assert!(
        elapsed < std::time::Duration::from_millis(100),
        "{elapsed:?}"
    );
}

/// 実機の config.toml を渡したときだけ走る、実カタログでの突き合わせ。
/// 開発機のインストール状況に依存するので、環境変数が無ければ skip する。
///
/// ```text
/// CMRT_TEST_DAW_REAL_CONFIG=%LOCALAPPDATA%/clap-mml-render-tui/config.toml ///   cargo test -p cmrt-daw real_catalog -- --nocapture
/// ```
#[test]
fn real_catalog_get_patches_returns_from_the_snapshot() {
    let Some(config_path) = std::env::var_os("CMRT_TEST_DAW_REAL_CONFIG") else {
        eprintln!("skip: CMRT_TEST_DAW_REAL_CONFIG が未設定");
        return;
    };
    let cfg = Config::load_from_path(std::path::Path::new(&config_path))
        .expect("CMRT_TEST_DAW_REAL_CONFIG の config.toml を読めること");

    let scan_started = std::time::Instant::now();
    let pairs = cmrt_tui_core::patches::collect_patch_pairs(&cfg).expect("実カタログの走査");
    let scan_elapsed = scan_started.elapsed();
    assert!(!pairs.is_empty(), "実機カタログが 0 件では比較にならない");

    let patch_load = Arc::new(Mutex::new(PatchLoadState::ready(pairs.clone())));
    let started = std::time::Instant::now();
    let names = http_patch_names(&cfg, Some(&patch_load)).expect("snapshot 経路なら成功する");
    let snapshot_elapsed = started.elapsed();

    assert_eq!(names.len(), pairs.len());
    eprintln!(
        "real catalog: count={} scan={scan_elapsed:?} snapshot(get-patches)={snapshot_elapsed:?}",
        pairs.len()
    );
    assert!(
        snapshot_elapsed * 5 < scan_elapsed,
        "snapshot 経路が走査より十分速くない: scan={scan_elapsed:?} snapshot={snapshot_elapsed:?}"
    );
}
