//! `r`（ランダム音色）が、注入された patch catalog snapshot を使い
//! **音色 file を走査しない**ことの検証。
//!
//! 走査経路との区別は「実在しない patch dir を指す Config」で作る。
//! 走査なら候補 0 件で `Ok(false)`、snapshot 経路なら `Ok(true)` になるので、
//! **走査していないことを戻り値だけで判定できる**（所要時間の閾値に頼らない）。

use super::*;

use cmrt_tui_core::patch_load::PatchLoadState;

/// 実在しない patch dir を指す Config へ差し替える。
fn point_config_at_missing_patch_dir(app: &mut DawApp) {
    let missing = std::env::temp_dir().join("cmrt_test_daw_missing_patch_dir_does_not_exist");
    std::fs::remove_dir_all(&missing).ok();
    app.cfg = Arc::new(Config {
        patches_dirs: Some(vec![missing.to_string_lossy().into_owned()]),
        ..(*app.cfg).clone()
    });
}

fn snapshot_pairs() -> Vec<(String, String)> {
    vec![(
        "Pads/Snapshot Pad.fxp".to_string(),
        "pads/snapshot pad.fxp".to_string(),
    )]
}

fn log_lines(app: &DawApp) -> Vec<String> {
    app.log_lines.lock().unwrap().iter().cloned().collect()
}

#[test]
fn apply_random_patch_to_track_uses_injected_snapshot_instead_of_scanning() {
    let tmp = std::env::temp_dir().join("cmrt_test_daw_random_patch_snapshot");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(&tmp).unwrap();

    {
        let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);

        let (mut app, _cache_rx) = build_test_app();
        point_config_at_missing_patch_dir(&mut app);
        *app.patch_load.lock().unwrap() = PatchLoadState::ready(snapshot_pairs());

        let started = std::time::Instant::now();
        let applied = app.apply_random_patch_to_track(1);
        let elapsed = started.elapsed();

        assert_eq!(
            applied,
            Ok(true),
            "snapshot が Ready なら、走査できない Config でも音色が決まるはず"
        );
        assert!(
            app.editor.data[1][0].contains("Pads/Snapshot Pad.fxp"),
            "init セルへ snapshot の音色が入るはず: {}",
            app.editor.data[1][0]
        );
        // 走査経路は実測 1.3 秒。snapshot 経路なら桁違いに速い。
        assert!(
            elapsed < std::time::Duration::from_millis(100),
            "snapshot 経路が遅すぎる: {elapsed:?}"
        );
        assert!(
            log_lines(&app)
                .iter()
                .any(|line| line.contains("daw: event=patch-pairs source=snapshot count=1")),
            "snapshot 経路を通ったログが残るはず: {:?}",
            log_lines(&app)
        );
    }

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn apply_random_patch_to_track_falls_back_to_scan_while_catalog_is_loading() {
    let tmp = std::env::temp_dir().join("cmrt_test_daw_random_patch_loading_fallback");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(&tmp).unwrap();

    {
        let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);

        let (mut app, _cache_rx) = build_test_app();
        point_config_at_missing_patch_dir(&mut app);
        assert!(
            matches!(*app.patch_load.lock().unwrap(), PatchLoadState::Loading),
            "既定の test app は Loading で始まる"
        );

        let applied = app.apply_random_patch_to_track(1);

        assert_eq!(
            applied,
            Ok(false),
            "Loading の間は走査へフォールバックし、候補 0 件なら no-op"
        );
        assert!(
            log_lines(&app)
                .iter()
                .any(|line| line.contains("daw: event=patch-pairs source=scan")),
            "走査経路を通ったログが残るはず: {:?}",
            log_lines(&app)
        );
    }

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn catalog_patch_pairs_is_none_unless_the_snapshot_is_ready() {
    let (app, _cache_rx) = build_test_app();

    assert!(app.catalog_patch_pairs().is_none(), "Loading では None");

    *app.patch_load.lock().unwrap() = PatchLoadState::Err("boom".to_string());
    assert!(app.catalog_patch_pairs().is_none(), "Err でも None");

    *app.patch_load.lock().unwrap() = PatchLoadState::ready(snapshot_pairs());
    assert_eq!(app.catalog_patch_pairs(), Some(snapshot_pairs()));
}

/// 実機の config.toml を渡したときだけ走る、実カタログでの突き合わせ。
///
/// 開発機のインストール状況に依存するので、環境変数が無ければ skip する
/// （個人のパスをコードへ書かないため、パスは環境変数で渡す）。
///
/// ```text
/// CMRT_TEST_DAW_REAL_CONFIG=%LOCALAPPDATA%\clap-mml-render-tui\config.toml \
///   cargo test -p cmrt-daw real_catalog -- --nocapture
/// ```
#[test]
fn real_catalog_snapshot_path_beats_the_scan_path() {
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

    let tmp = std::env::temp_dir().join("cmrt_test_daw_real_catalog_snapshot");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(&tmp).unwrap();
    let (snapshot_elapsed, count) = {
        let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);

        let (mut app, _cache_rx) = build_test_app();
        app.cfg = Arc::new(cfg);
        *app.patch_load.lock().unwrap() = PatchLoadState::ready(pairs.clone());

        let started = std::time::Instant::now();
        assert_eq!(app.apply_random_patch_to_track(1), Ok(true));
        (started.elapsed(), pairs.len())
    };
    std::fs::remove_dir_all(&tmp).ok();

    eprintln!(
        "real catalog: count={count} scan={scan_elapsed:?} snapshot(apply)={snapshot_elapsed:?}"
    );
    assert!(
        snapshot_elapsed * 5 < scan_elapsed,
        "snapshot 経路が走査より十分速くない: scan={scan_elapsed:?} snapshot={snapshot_elapsed:?}"
    );
}
