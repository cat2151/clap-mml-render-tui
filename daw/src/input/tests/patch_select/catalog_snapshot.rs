//! 音色選択 overlay が、注入された patch catalog snapshot を使い
//! **開くたびに音色 file を走査しない**ことの検証。
//!
//! 走査経路との区別は Stage 1 と同じ「実在しない patch dir を指す Config」で作る。
//! 走査なら 0 件で開けないので、一覧が埋まること自体が走査していない証拠になる。

use super::*;

use cmrt_tui_core::patch_load::PatchLoadState;

fn point_config_at_missing_patch_dir(app: &mut DawApp) {
    let missing = std::env::temp_dir().join("cmrt_test_daw_patch_select_missing_dir_absent");
    std::fs::remove_dir_all(&missing).ok();
    app.cfg = Arc::new(Config {
        patches_dirs: Some(vec![missing.to_string_lossy().into_owned()]),
        ..(*app.cfg).clone()
    });
}

fn snapshot_pairs() -> Vec<(String, String)> {
    vec![
        (
            "Bass/Snapshot Bass.fxp".to_string(),
            "bass/snapshot bass.fxp".to_string(),
        ),
        (
            "Pads/Snapshot Pad.fxp".to_string(),
            "pads/snapshot pad.fxp".to_string(),
        ),
    ]
}

fn log_lines(app: &DawApp) -> Vec<String> {
    app.log_lines.lock().unwrap().iter().cloned().collect()
}

#[test]
fn start_patch_select_overlay_uses_injected_snapshot_instead_of_scanning() {
    let (mut app, _cache_rx) = build_test_app();
    point_config_at_missing_patch_dir(&mut app);
    *app.patch_load.lock().unwrap() = PatchLoadState::ready(snapshot_pairs());
    app.editor.cursor_track = 1;

    let started = std::time::Instant::now();
    app.start_patch_select_overlay(None);
    let elapsed = started.elapsed();

    assert!(matches!(app.mode, DawMode::PatchSelect));
    assert_eq!(
        app.overlays.patch_select.filtered,
        vec![
            "Bass/Snapshot Bass.fxp".to_string(),
            "Pads/Snapshot Pad.fxp".to_string()
        ],
        "snapshot が Ready なら、走査できない Config でも一覧が埋まるはず"
    );
    // 走査経路は実測 1.3 秒。snapshot 経路なら桁違いに速い。
    assert!(
        elapsed < std::time::Duration::from_millis(100),
        "snapshot 経路が遅すぎる: {elapsed:?}"
    );
    assert!(
        log_lines(&app).iter().any(|line| line
            .contains("daw: event=patch-pairs source=snapshot count=2")
            && line.contains("reason=patch-select-overlay")),
        "snapshot 経路を通ったログが残るはず: {:?}",
        log_lines(&app)
    );
}

#[test]
fn start_patch_select_overlay_falls_back_to_scan_while_catalog_is_loading() {
    let (mut app, _cache_rx) = build_test_app();
    point_config_at_missing_patch_dir(&mut app);
    app.editor.cursor_track = 1;

    app.start_patch_select_overlay(None);

    assert!(
        matches!(app.mode, DawMode::Normal),
        "走査経路では 0 件なので overlay は開かない"
    );
    assert!(
        log_lines(&app)
            .iter()
            .any(|line| line.contains("daw: event=patch-pairs source=scan")
                && line.contains("reason=patch-select-overlay")),
        "走査経路を通ったログが残るはず: {:?}",
        log_lines(&app)
    );
    assert!(
        log_lines(&app)
            .iter()
            .any(|line| line == "パッチの読み込みに失敗しました"),
        "実在しない patch dir の走査は Err になる: {:?}",
        log_lines(&app)
    );
}

/// `patches_dirs` 未設定 + snapshot 無しでは走査しない。
///
/// 走査すると `catalog_plugins()` が拾う「この開発機にインストール済みのプラグイン」の
/// 音色が数千件混ざり、テストがマシン依存になる。
#[test]
fn start_patch_select_overlay_reports_unset_patch_dirs_without_scanning() {
    let (mut app, _cache_rx) = build_test_app();
    app.cfg = Arc::new(Config {
        patches_dirs: None,
        ..(*app.cfg).clone()
    });
    app.editor.cursor_track = 1;

    let started = std::time::Instant::now();
    app.start_patch_select_overlay(None);
    let elapsed = started.elapsed();

    assert!(matches!(app.mode, DawMode::Normal));
    assert!(
        log_lines(&app)
            .iter()
            .any(|line| line == "patches_dirs が設定されていません"),
        "{:?}",
        log_lines(&app)
    );
    assert!(
        elapsed < std::time::Duration::from_millis(100),
        "走査してしまっている: {elapsed:?}"
    );
}

/// 実機の config.toml を渡したときだけ走る、実カタログでの突き合わせ。
///
/// 開発機のインストール状況に依存するので、環境変数が無ければ skip する
/// （個人のパスをコードへ書かないため、パスは環境変数で渡す）。
///
/// ```text
/// CMRT_TEST_DAW_REAL_CONFIG=%LOCALAPPDATA%/clap-mml-render-tui/config.toml ///   cargo test -p cmrt-daw real_catalog -- --nocapture
/// ```
#[test]
fn real_catalog_patch_select_overlay_opens_without_scanning() {
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

    let (mut app, _cache_rx) = build_test_app();
    app.cfg = Arc::new(cfg);
    *app.patch_load.lock().unwrap() = PatchLoadState::ready(pairs.clone());
    app.editor.cursor_track = 1;

    let started = std::time::Instant::now();
    app.start_patch_select_overlay(None);
    let snapshot_elapsed = started.elapsed();

    assert!(matches!(app.mode, DawMode::PatchSelect));
    assert_eq!(app.overlays.patch_select.filtered.len(), pairs.len());
    eprintln!(
        "real catalog: count={} scan={scan_elapsed:?} snapshot(patch-select)={snapshot_elapsed:?}",
        pairs.len()
    );
    assert!(
        snapshot_elapsed * 5 < scan_elapsed,
        "snapshot 経路が走査より十分速くない: scan={scan_elapsed:?} snapshot={snapshot_elapsed:?}"
    );
}
