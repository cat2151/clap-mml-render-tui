//! HTTP 経路（`POST /patch` / `GET /patches`）が、注入された patch catalog snapshot を使い
//! **音色 file を走査しない**ことの検証。
//!
//! 走査経路との区別は他 Stage と同じ「実在しない patch dir を指す Config」で作る。

use super::*;

use cmrt_tui_core::patch_load::PatchLoadState;

fn cfg_pointing_at_missing_patch_dir() -> Config {
    let missing = std::env::temp_dir().join("cmrt_test_http_patch_missing_dir_absent");
    std::fs::remove_dir_all(&missing).ok();
    Config {
        patches_dirs: Some(vec![missing.to_string_lossy().into_owned()]),
        ..default_config()
    }
}

#[test]
fn apply_pending_http_patch_command_uses_the_injected_snapshot() {
    let _test_guard = lock_http_server_test_state();
    let tmp = std::env::temp_dir().join("cmrt_test_http_server_patch_snapshot");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(&tmp).unwrap();
    let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);

    let cfg = cfg_pointing_at_missing_patch_dir();
    let state = build_http_state(cfg.clone());
    activate_http_state(Arc::clone(&state));
    let response_rx = enqueue_command(
        &state,
        DawHttpCommandKind::Patch {
            track: 1,
            patch: "Pads/Snapshot Pad.fxp".to_string(),
        },
    );

    let mut app = build_test_app(cfg);
    *app.patch_load.lock().unwrap() = PatchLoadState::ready(vec![(
        "Pads/Snapshot Pad.fxp".to_string(),
        "pads/snapshot pad.fxp".to_string(),
    )]);

    let started = std::time::Instant::now();
    app.apply_pending_http_commands();
    let elapsed = started.elapsed();

    assert_eq!(
        app.editor.data[1][0],
        DawApp::build_patch_json("Pads/Snapshot Pad.fxp"),
        "snapshot 由来の表示パス全文へ解決されるはず"
    );
    assert_eq!(response_rx.try_recv().unwrap(), Ok(()));
    // 走査経路は実測 1.3 秒。snapshot 経路なら桁違いに速い。
    assert!(
        elapsed < std::time::Duration::from_millis(100),
        "snapshot 経路が遅すぎる: {elapsed:?}"
    );
    assert!(
        app.log_lines.lock().unwrap().iter().any(|line| line
            .contains("daw: event=patch-pairs source=snapshot count=1")
            && line.contains("reason=http-post-patch")),
        "snapshot 経路を通ったログが残るはず: {:?}",
        app.log_lines.lock().unwrap()
    );

    deactivate_daw_http_server();
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn apply_pending_http_patch_command_falls_back_to_scan_while_catalog_is_loading() {
    let _test_guard = lock_http_server_test_state();
    let tmp = std::env::temp_dir().join("cmrt_test_http_server_patch_scan_fallback");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(&tmp).unwrap();
    let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);

    let cfg = cfg_pointing_at_missing_patch_dir();
    let state = build_http_state(cfg.clone());
    activate_http_state(Arc::clone(&state));
    let response_rx = enqueue_command(
        &state,
        DawHttpCommandKind::Patch {
            track: 1,
            patch: "Pads/Snapshot Pad.fxp".to_string(),
        },
    );

    let mut app = build_test_app(cfg);
    app.apply_pending_http_commands();

    assert_eq!(
        response_rx.try_recv().unwrap(),
        Err("patch 一覧の取得に失敗しました".to_string()),
        "走査経路では実在しない patch dir の走査が失敗する"
    );
    assert!(
        app.log_lines
            .lock()
            .unwrap()
            .iter()
            .any(|line| line.contains("daw: event=patch-pairs source=scan")
                && line.contains("reason=http-post-patch")),
        "走査経路を通ったログが残るはず: {:?}",
        app.log_lines.lock().unwrap()
    );

    deactivate_daw_http_server();
    std::fs::remove_dir_all(&tmp).ok();
}
