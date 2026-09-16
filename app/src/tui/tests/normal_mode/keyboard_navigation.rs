use super::*;
use crate::tui::keyboard::PatchPaneFocus;

fn press(app: &mut TuiApp<'_>, code: KeyCode, modifiers: KeyModifiers) {
    app.handle_keyboard_key_event(KeyEvent::new(code, modifiers));
}

fn numbered_patches(categories: &[(&str, usize)]) -> Vec<(String, String)> {
    categories
        .iter()
        .flat_map(|(category, count)| {
            (0..*count).map(move |index| {
                let patch = format!("patches_factory/{category}/{category} {index:02}.fxp");
                let normalized = patch.to_lowercase();
                (patch, normalized)
            })
        })
        .collect()
}

fn keyboard_app(categories: &[(&str, usize)], patch: &str) -> TuiApp<'static> {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::ready(numbered_patches(
        categories,
    ))));
    app.start_keyboard(Some(patch.to_string()));
    app
}

#[test]
fn start_keyboard_from_notepad_uses_current_cursor_line_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.notepad.set_session_lines_for_test(vec![
        r#"{"Surge XT patch":"Pads/First.fxp"} c"#.to_string(),
        r#"{"Surge XT patch":"Keys/Current.fxp"} d"#.to_string(),
        r#"{"Surge XT patch":"Leads/Last.fxp"} e"#.to_string(),
    ]);
    app.notepad.set_session_cursor_for_test(1);

    app.start_keyboard_from_notepad();

    assert_eq!(
        app.active_screen,
        crate::screen_switch::PrimaryScreen::Keyboard
    );
    assert_eq!(app.keyboard.state.patch(), Some("Keys/Current.fxp"));
}

#[test]
fn start_keyboard_from_notepad_uses_init_saw_without_valid_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.notepad
        .set_session_lines_for_test(vec![r#"{"Surge XT patch":""} c"#.to_string()]);

    app.start_keyboard_from_notepad();

    assert_eq!(
        app.active_screen,
        crate::screen_switch::PrimaryScreen::Keyboard
    );
    assert_eq!(app.keyboard.state.patch(), None);
}

#[test]
fn keyboard_counted_j_and_k_accept_multi_digit_prefixes() {
    let mut app = keyboard_app(&[("Lead", 15)], "patches_factory/Lead/Lead 00.fxp");

    press(&mut app, KeyCode::Char('1'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('1'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 11.fxp")
    );

    press(&mut app, KeyCode::Char('2'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('k'), KeyModifiers::NONE);
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 09.fxp")
    );
    assert_eq!(app.keyboard.state.navigation_count.value(), None);
}

#[test]
fn keyboard_counted_h_and_l_move_the_pane_focus_and_stop_at_both_ends() {
    let mut app = keyboard_app(
        &[("Lead", 1), ("Pad", 1), ("Strings", 1)],
        "patches_factory/Lead/Lead 00.fxp",
    );
    let focus = |app: &TuiApp<'_>| app.keyboard.state.patch_catalog.focus();
    assert_eq!(focus(&app), PatchPaneFocus::Patches);

    press(&mut app, KeyCode::Char('l'), KeyModifiers::NONE);
    assert_eq!(focus(&app), PatchPaneFocus::Patches);

    press(&mut app, KeyCode::Char('2'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('h'), KeyModifiers::NONE);
    assert_eq!(focus(&app), PatchPaneFocus::Role);

    press(&mut app, KeyCode::Char('h'), KeyModifiers::NONE);
    assert_eq!(focus(&app), PatchPaneFocus::Role);

    press(&mut app, KeyCode::Char('l'), KeyModifiers::NONE);
    assert_eq!(focus(&app), PatchPaneFocus::Preset);
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 00.fxp")
    );
}

#[test]
fn keyboard_j_and_k_move_the_focused_pane() {
    let mut app = keyboard_app(
        &[("Leads", 2), ("Pads", 2), ("Strings", 2), ("Basses", 2)],
        "patches_factory/Leads/Leads 00.fxp",
    );
    let role = |app: &TuiApp<'_>| {
        let catalog = &app.keyboard.state.patch_catalog;
        catalog.roles()[catalog.role_cursor()].label()
    };
    let preset = |app: &TuiApp<'_>| {
        let catalog = &app.keyboard.state.patch_catalog;
        catalog.presets()[catalog.preset_cursor()].label.clone()
    };

    press(&mut app, KeyCode::Char('h'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('h'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(role(&app), "Bass track");
    assert_eq!(preset(&app), "ALL");
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Basses/Basses 00.fxp")
    );

    press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(role(&app), "Chord track");
    assert_eq!(preset(&app), "ALL");
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Pads/Pads 00.fxp")
    );

    press(&mut app, KeyCode::Char('l'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(role(&app), "Chord track");
    assert_eq!(preset(&app), "strings");
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Strings/Strings 00.fxp")
    );

    press(&mut app, KeyCode::Char('k'), KeyModifiers::NONE);
    assert_eq!(preset(&app), "ALL");
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Strings/Strings 00.fxp")
    );
}

#[test]
fn keyboard_r_stays_inside_the_selected_role_and_focuses_patches() {
    let mut app = keyboard_app(
        &[("Basses", 5), ("Leads", 5), ("Pads", 5)],
        "patches_factory/Leads/Leads 00.fxp",
    );

    press(&mut app, KeyCode::Char('h'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('h'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(
        app.keyboard.state.patch_catalog.focus(),
        PatchPaneFocus::Role
    );

    for _ in 0..10 {
        press(&mut app, KeyCode::Char('r'), KeyModifiers::NONE);
        let patch = app.keyboard.state.patch().unwrap();
        assert!(
            patch.starts_with("patches_factory/Basses/"),
            "random left the Bass role: {patch}"
        );
        assert_eq!(
            app.keyboard.state.patch_catalog.focus(),
            PatchPaneFocus::Patches
        );
    }
}

#[test]
fn keyboard_count_multiplies_ctrl_page_movement() {
    let mut app = keyboard_app(&[("Pad", 25)], "patches_factory/Pad/Pad 00.fxp");

    press(&mut app, KeyCode::Char('2'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('d'), KeyModifiers::CONTROL);
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Pad/Pad 20.fxp")
    );

    press(&mut app, KeyCode::Char('2'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('u'), KeyModifiers::CONTROL);
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Pad/Pad 00.fxp")
    );
}

#[test]
fn keyboard_non_vim_keys_clear_count_and_keep_their_normal_behavior() {
    let mut app = keyboard_app(&[("Lead", 6)], "patches_factory/Lead/Lead 00.fxp");

    press(&mut app, KeyCode::Char('3'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 01.fxp")
    );

    press(&mut app, KeyCode::Char('2'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('v'), KeyModifiers::NONE);
    assert_eq!(app.keyboard.state.velocity(), 127);
    assert_eq!(app.keyboard.state.navigation_count.value(), None);

    press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 02.fxp")
    );
}

#[test]
fn keyboard_bare_zero_is_not_a_count() {
    let mut app = keyboard_app(&[("Lead", 3)], "patches_factory/Lead/Lead 00.fxp");

    press(&mut app, KeyCode::Char('0'), KeyModifiers::NONE);

    assert_eq!(app.keyboard.state.navigation_count.value(), None);
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 00.fxp")
    );
}

#[test]
fn keyboard_huge_count_clamps_without_overflowing() {
    let mut app = keyboard_app(&[("Lead", 3)], "patches_factory/Lead/Lead 01.fxp");

    for _ in 0..100 {
        press(&mut app, KeyCode::Char('9'), KeyModifiers::NONE);
    }
    press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 02.fxp")
    );

    for _ in 0..100 {
        press(&mut app, KeyCode::Char('9'), KeyModifiers::NONE);
    }
    press(&mut app, KeyCode::Char('k'), KeyModifiers::NONE);
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 00.fxp")
    );
}
