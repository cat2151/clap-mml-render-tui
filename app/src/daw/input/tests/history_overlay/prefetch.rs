use super::*;

#[test]
fn handle_history_overlay_j_prefetches_predicted_preview_cache() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 1;
    app.editor.data[1][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#.to_string();
    app.patch_phrase_store.patches.insert(
        "Bass/Bass 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec![
                "bass zero".to_string(),
                "bass one".to_string(),
                "bass two".to_string(),
            ],
            favorites: vec![],
        },
    );
    app.start_history_overlay_for_patch_name(Some("Bass/Bass 1.fxp".to_string()));

    app.handle_history_overlay(KeyCode::Char('j'));

    assert_eq!(app.playback.overlay_preview_cache.lock().unwrap().len(), 2);
}

#[test]
fn prefetch_preview_snapshot_skips_overlay_cache_for_large_measure_buffers() {
    let (app, _cache_rx) = build_test_app();
    let mut app = app;
    app.editor.data[0][0] = r#"{"beat":"4/4"}t1"#.to_string();
    app.editor.data[1][1] = "c".to_string();

    app.prefetch_preview_snapshot(
        0,
        app.build_measure_track_mmls_for_measure(1),
        vec![0.0, 1.0, 0.0],
    );

    assert!(app
        .playback
        .overlay_preview_cache
        .lock()
        .unwrap()
        .is_empty());
}
