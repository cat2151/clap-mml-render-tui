use super::*;

#[test]
fn handle_notepad_history_n_p_t_switch_to_corresponding_overlays() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Line Patch"} line phrase"#.to_string()];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "Line Patch",
        "Pads/Pad 1.fxp",
    ]))));
    app.patch_phrase_store.notepad.history = vec![
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"} selected phrase"#.to_string(),
        "plain phrase".to_string(),
    ];
    app.patch_phrase_store.notepad.favorites = vec!["favorite".to_string()];
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        cmrt_history::PatchPhraseState {
            history: vec!["selected phrase".to_string()],
            favorites: vec!["fav".to_string()],
        },
    );
    app.start_notepad_history();

    // overlay 切替キーを統一するため、notepad history 中でも n で先頭選択の初期状態に戻せるようにする。
    app.handle_notepad_history(KeyCode::Char('n'));
    assert!(matches!(app.mode, Mode::NotepadHistory));
    assert_eq!(app.notepad_history.history_cursor, 0);

    app.start_notepad_history();
    app.handle_notepad_history(KeyCode::Char('p'));
    assert!(matches!(app.mode, Mode::PatchPhrase));
    assert_eq!(
        app.patch_phrase.patch_name.as_deref(),
        Some("Pads/Pad 1.fxp")
    );

    app.start_notepad_history();
    app.handle_notepad_history(KeyCode::Char('t'));
    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(
        app.patch_select.patch_filtered[app.patch_select.patch_cursor],
        "Pads/Pad 1.fxp"
    );
}
