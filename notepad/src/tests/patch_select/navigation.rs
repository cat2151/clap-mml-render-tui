use super::*;

fn tone_mml(patch: &str) -> String {
    format!(r#"{{"Surge XT patch": "{patch}"}} l8cdef"#)
}

#[test]
fn j_and_k_move_the_patch_cursor_and_preview_the_destination() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones(&mut app, 3, "Tone 01");

    press(&mut app, KeyCode::Char('j'));
    assert_eq!(selected(&app).as_deref(), Some("Tone 02"));
    assert_eq!(playing(&app), Some(tone_mml("Tone 02")));

    press(&mut app, KeyCode::Char('k'));
    assert_eq!(selected(&app).as_deref(), Some("Tone 01"));
    assert_eq!(playing(&app), Some(tone_mml("Tone 01")));
}

#[test]
fn ctrl_j_n_move_down_and_ctrl_k_p_move_up() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones(&mut app, 4, "Tone 01");

    press_ctrl(&mut app, 'j');
    assert_eq!(selected(&app).as_deref(), Some("Tone 02"));
    press_ctrl(&mut app, 'n');
    assert_eq!(selected(&app).as_deref(), Some("Tone 03"));
    assert_eq!(playing(&app), Some(tone_mml("Tone 03")));
    press_ctrl(&mut app, 'k');
    assert_eq!(selected(&app).as_deref(), Some("Tone 02"));
    press_ctrl(&mut app, 'p');
    assert_eq!(selected(&app).as_deref(), Some("Tone 01"));
}

#[test]
fn page_keys_move_by_the_shared_page_step_and_home_end_jump_to_the_ends() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones(&mut app, 12, "Tone 00");

    press(&mut app, KeyCode::PageDown);
    assert_eq!(selected(&app).as_deref(), Some("Tone 10"));
    press(&mut app, KeyCode::PageUp);
    assert_eq!(selected(&app).as_deref(), Some("Tone 00"));
    press(&mut app, KeyCode::End);
    assert_eq!(selected(&app).as_deref(), Some("Tone 11"));
    press(&mut app, KeyCode::Home);
    assert_eq!(selected(&app).as_deref(), Some("Tone 00"));
}

#[test]
fn ctrl_s_changes_nothing() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_tones(&mut app, 3, "Tone 01");
    let list = patch_select_list(&app);
    let played = playing(&app);

    press_ctrl(&mut app, 's');

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(patch_select_list(&app), list);
    assert_eq!(selected(&app).as_deref(), Some("Tone 01"));
    assert_eq!(playing(&app), played);
}

#[test]
fn h_moves_to_the_preset_and_role_panes_that_change_the_list() {
    let mut app = NotepadScreen::new_for_test(test_config());
    open_patch_select_for_test(
        &mut app,
        r#"{"Surge XT patch":"Pads/Pad 1.fxp"} l8cdef"#,
        &["Pads/Pad 1.fxp", "Pads/Pad 2.fxp", "Leads/Lead 1.fxp"],
    );

    press(&mut app, KeyCode::Char('h'));
    press(&mut app, KeyCode::Home);
    assert_eq!(preset_label(&app), "ALL");
    assert_eq!(
        patch_select_list(&app),
        vec!["Pads/Pad 1.fxp", "Pads/Pad 2.fxp"]
    );

    press(&mut app, KeyCode::Char('h'));
    press(&mut app, KeyCode::Home);
    assert_eq!(patch_select_list(&app).len(), 3);
}
