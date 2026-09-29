use super::*;

#[test]
fn handle_normal_t_enters_patch_select_when_random_timbre_disabled() {
    let mut app = NotepadScreen::new_for_test(test_config());
    let patches = make_patches(&["Pads/Pad 1.fxp"]);
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::ready(patches)));

    app.handle_normal(KeyCode::Char('t'));

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(patch_select_list(&app), vec!["Pads/Pad 1.fxp"]);
}

/// 起動直後の読み込み中に押した `t` は捨てずに覚えておき、読み込み後に開く。
#[test]
fn t_while_the_catalog_is_loading_opens_patch_select_once_loaded() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":"Leads/Lead 1.fxp"} l8cdef"#.to_string()];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Loading));

    app.handle_normal(KeyCode::Char('t'));
    app.open_pending_patch_select_if_loaded();

    assert!(matches!(app.mode, Mode::Normal));
    assert!(app.patch_select_open_pending());
    assert!(!matches!(
        *app.playback.session.play_state().lock().unwrap(),
        PlayState::Err(_)
    ));

    *app.patch_load_state.lock().unwrap() =
        PatchLoadState::ready(make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]));
    app.open_pending_patch_select_if_loaded();

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert!(!app.patch_select_open_pending());
    assert_eq!(
        app.patch_select_selected_patch_name().as_deref(),
        Some("Leads/Lead 1.fxp")
    );
}

/// 待っている間に INSERT へ入ったら、その操作を優先して勝手に開かない。
#[test]
fn pending_patch_select_is_dropped_if_the_user_moved_on() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Loading));
    app.handle_normal(KeyCode::Char('t'));
    app.mode = Mode::Insert;

    *app.patch_load_state.lock().unwrap() =
        PatchLoadState::ready(make_patches(&["Pads/Pad 1.fxp"]));
    app.open_pending_patch_select_if_loaded();

    assert!(matches!(app.mode, Mode::Insert));
    assert!(!app.patch_select_open_pending());
}

#[test]
fn handle_normal_t_selects_current_line_patch_when_present() {
    let mut app = NotepadScreen::new_for_test(test_config());
    let patches = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    app.editor.lines = vec![r#"{"Surge XT patch":"Leads/Lead 1.fxp"} l8cdef"#.to_string()];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::ready(patches)));

    app.handle_normal(KeyCode::Char('t'));

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(
        app.patch_select_selected_patch_name().as_deref(),
        Some("Leads/Lead 1.fxp")
    );
}
