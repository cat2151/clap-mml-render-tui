use super::*;

#[test]
fn handle_normal_t_enters_patch_select_when_random_timbre_disabled() {
    let mut app = NotepadScreen::new_for_test(test_config());
    let patches = make_patches(&["Pads/Pad 1.fxp"]);
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(patches.clone())));

    app.handle_normal(KeyCode::Char('t'));

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(app.patch_select.patch_all, patches);
    assert_eq!(app.patch_select.patch_filtered, vec!["Pads/Pad 1.fxp"]);
}

#[test]
fn handle_normal_t_selects_current_line_patch_when_present() {
    let mut app = NotepadScreen::new_for_test(test_config());
    let patches = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    app.editor.lines = vec![r#"{"Surge XT patch":"Leads/Lead 1.fxp"} l8cdef"#.to_string()];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(patches)));

    app.handle_normal(KeyCode::Char('t'));

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(app.patch_select.patch_cursor, 1);
    assert_eq!(app.patch_select.patch_list_state.selected(), Some(1));
}
