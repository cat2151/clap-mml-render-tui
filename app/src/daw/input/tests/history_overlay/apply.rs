use super::*;

#[test]
fn handle_history_overlay_enter_overwrites_measure_and_backs_up_old_phrase() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 2;
    app.editor.data[1][0] = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#.to_string();
    app.editor.data[1][2] = "before".to_string();
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["after".to_string()],
            favorites: vec![],
        },
    );
    app.start_history_overlay();

    app.handle_history_overlay(KeyCode::Enter);

    assert!(matches!(app.mode, DawMode::Normal));
    assert_eq!(app.editor.data[1][2], "after");
    assert_eq!(
        app.patch_phrase_store
            .patches
            .get("Pads/Pad 1.fxp")
            .expect("patch history")
            .history,
        vec!["before".to_string(), "after".to_string()]
    );
    assert!(app.patch_phrase_store_dirty);
}

#[test]
fn handle_history_overlay_enter_without_track_patch_sets_patch_and_backs_up_old_phrase() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 2;
    app.editor.data[1][2] = "before".to_string();
    app.patch_phrase_store.notepad.history =
        vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()];
    app.start_history_overlay();

    app.handle_history_overlay(KeyCode::Enter);

    assert!(matches!(app.mode, DawMode::Normal));
    assert_eq!(
        app.editor.data[1][0],
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"}"#
    );
    assert_eq!(app.editor.data[1][2], "l8cdef");
    assert_eq!(
        app.patch_phrase_store.notepad.history,
        vec![
            r#"{"Surge XT patch":"Pads/Pad 1.fxp"} before"#.to_string(),
            r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#.to_string()
        ]
    );
    assert!(app.patch_phrase_store_dirty);
}

#[test]
fn handle_history_overlay_enter_from_favorites_uses_selected_favorite() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 1;
    app.editor.data[1][0] = r#"{"Surge XT patch": "Pads/Pad 1.fxp"}"#.to_string();
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        crate::history::PatchPhraseState {
            history: vec!["history".to_string()],
            favorites: vec!["favorite".to_string()],
        },
    );
    app.start_history_overlay();
    app.handle_history_overlay(KeyCode::Char('l'));

    app.handle_history_overlay(KeyCode::Enter);

    assert_eq!(app.editor.data[1][1], "favorite");
}
