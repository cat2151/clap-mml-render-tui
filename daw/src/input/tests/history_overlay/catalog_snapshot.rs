//! フレーズ履歴 overlay の音色名解決が、注入された patch catalog snapshot を使い
//! **音色 file を走査しない**ことの検証。

use super::*;

use cmrt_tui_core::patch_load::PatchLoadState;

fn log_lines(app: &DawApp) -> Vec<String> {
    app.log_lines.lock().unwrap().iter().cloned().collect()
}

/// 走査できない Config でも、snapshot にある表示パス全文へ解決できること。
///
/// 解決できたこと自体が「snapshot を見た」証拠になる（走査経路なら 0 件で解決不能）。
#[test]
fn history_overlay_resolves_patch_name_from_the_injected_snapshot() {
    let missing = std::env::temp_dir().join("cmrt_test_daw_history_missing_patch_dir_absent");
    std::fs::remove_dir_all(&missing).ok();

    let (mut app, _cache_rx) = build_test_app();
    app.cfg = Arc::new(Config {
        patches_dirs: Some(vec![missing.to_string_lossy().into_owned()]),
        ..(*app.cfg).clone()
    });
    *app.patch_load.lock().unwrap() = PatchLoadState::ready(vec![(
        "patches_factory/Pads/Pad 1.fxp".to_string(),
        "patches_factory/pads/pad 1.fxp".to_string(),
    )]);
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 2;
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        cmrt_history::PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );

    let started = std::time::Instant::now();
    app.start_history_overlay_for_patch_name(Some("Pads/Pad 1.fxp".to_string()));
    let elapsed = started.elapsed();

    assert!(matches!(app.mode, DawMode::History));
    assert_eq!(
        app.overlays.history.patch_name.as_deref(),
        Some("patches_factory/Pads/Pad 1.fxp"),
        "snapshot 由来の表示パス全文へ解決されるはず"
    );
    assert!(app
        .patch_phrase_store
        .patches
        .contains_key("patches_factory/Pads/Pad 1.fxp"));
    // 走査経路は実測 1.3 秒。snapshot 経路なら桁違いに速い。
    assert!(
        elapsed < std::time::Duration::from_millis(100),
        "snapshot 経路が遅すぎる: {elapsed:?}"
    );
    assert!(
        log_lines(&app).iter().any(|line| line
            .contains("daw: event=patch-pairs source=snapshot count=1")
            && line.contains("reason=history-overlay")),
        "snapshot 経路を通ったログが残るはず: {:?}",
        log_lines(&app)
    );
}

/// `patches_dirs` 未設定 + snapshot 無しでは走査しないこと。
#[test]
fn history_overlay_does_not_scan_when_patch_dirs_are_unset() {
    let (mut app, _cache_rx) = build_test_app();
    app.cfg = Arc::new(Config {
        patches_dirs: None,
        ..(*app.cfg).clone()
    });
    app.editor.cursor_track = 2;

    let started = std::time::Instant::now();
    app.start_history_overlay_for_patch_name(Some("Pads/Pad 1.fxp".to_string()));
    let elapsed = started.elapsed();

    assert!(matches!(app.mode, DawMode::History));
    assert_eq!(
        app.overlays.history.patch_name.as_deref(),
        Some("Pads/Pad 1.fxp"),
        "解決先が無いときは正規化しただけの名前のまま"
    );
    assert!(
        elapsed < std::time::Duration::from_millis(100),
        "走査してしまっている: {elapsed:?}"
    );
}
