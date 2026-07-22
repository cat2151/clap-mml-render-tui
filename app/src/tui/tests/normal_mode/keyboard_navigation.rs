use super::*;

fn press(app: &mut TuiApp<'_>, code: KeyCode, modifiers: KeyModifiers) {
    app.handle_keyboard_key_event(KeyEvent::new(code, modifiers));
}

fn numbered_patches(categories: &[(&str, usize)]) -> Vec<(String, String)> {
    categories
        .iter()
        .flat_map(|(category, count)| {
            (0..*count).map(move |index| {
                let patch = format!("patches_factory/{category}/{category} {index}.fxp");
                let normalized = patch.to_lowercase();
                (patch, normalized)
            })
        })
        .collect()
}

fn keyboard_app(categories: &[(&str, usize)], patch: &str) -> TuiApp<'static> {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(numbered_patches(
        categories,
    ))));
    app.start_keyboard(Some(patch.to_string()));
    app
}

#[test]
fn start_keyboard_from_notepad_uses_current_cursor_line_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![
        r#"{"Surge XT patch":"Pads/First.fxp"} c"#.to_string(),
        r#"{"Surge XT patch":"Keys/Current.fxp"} d"#.to_string(),
        r#"{"Surge XT patch":"Leads/Last.fxp"} e"#.to_string(),
    ];
    app.editor.cursor = 1;

    app.start_keyboard_from_notepad();

    assert!(matches!(app.mode, Mode::Keyboard));
    assert_eq!(app.keyboard.state.patch(), Some("Keys/Current.fxp"));
}

#[test]
fn start_keyboard_from_notepad_uses_init_saw_without_valid_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.editor.lines = vec![r#"{"Surge XT patch":""} c"#.to_string()];

    app.start_keyboard_from_notepad();

    assert!(matches!(app.mode, Mode::Keyboard));
    assert_eq!(app.keyboard.state.patch(), None);
}

#[test]
fn keyboard_counted_j_and_k_accept_multi_digit_prefixes() {
    let mut app = keyboard_app(&[("Lead", 15)], "patches_factory/Lead/Lead 0.fxp");

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
        Some("patches_factory/Lead/Lead 9.fxp")
    );
    assert_eq!(app.keyboard.state.navigation_count.value(), None);
}

#[test]
fn keyboard_counted_h_and_l_move_categories() {
    let mut app = keyboard_app(
        &[("Lead", 1), ("Pad", 1), ("Strings", 1)],
        "patches_factory/Lead/Lead 0.fxp",
    );

    press(&mut app, KeyCode::Char('2'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('l'), KeyModifiers::NONE);
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Strings/Strings 0.fxp")
    );

    press(&mut app, KeyCode::Char('2'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('h'), KeyModifiers::NONE);
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 0.fxp")
    );
}

#[test]
fn keyboard_count_multiplies_ctrl_page_movement() {
    let mut app = keyboard_app(&[("Pad", 25)], "patches_factory/Pad/Pad 0.fxp");

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
        Some("patches_factory/Pad/Pad 0.fxp")
    );
}

#[test]
fn keyboard_non_vim_keys_clear_count_and_keep_their_normal_behavior() {
    let mut app = keyboard_app(&[("Lead", 6)], "patches_factory/Lead/Lead 0.fxp");

    press(&mut app, KeyCode::Char('3'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Down, KeyModifiers::NONE);
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 1.fxp")
    );

    press(&mut app, KeyCode::Char('2'), KeyModifiers::NONE);
    press(&mut app, KeyCode::Char('v'), KeyModifiers::NONE);
    assert_eq!(app.keyboard.state.velocity(), 127);
    assert_eq!(app.keyboard.state.navigation_count.value(), None);

    press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 2.fxp")
    );
}

#[test]
fn keyboard_bare_zero_is_not_a_count() {
    let mut app = keyboard_app(&[("Lead", 3)], "patches_factory/Lead/Lead 0.fxp");

    press(&mut app, KeyCode::Char('0'), KeyModifiers::NONE);

    assert_eq!(app.keyboard.state.navigation_count.value(), None);
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 0.fxp")
    );
}

#[test]
fn keyboard_huge_count_clamps_without_overflowing() {
    let mut app = keyboard_app(&[("Lead", 3)], "patches_factory/Lead/Lead 1.fxp");

    for _ in 0..100 {
        press(&mut app, KeyCode::Char('9'), KeyModifiers::NONE);
    }
    press(&mut app, KeyCode::Char('j'), KeyModifiers::NONE);
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 2.fxp")
    );

    for _ in 0..100 {
        press(&mut app, KeyCode::Char('9'), KeyModifiers::NONE);
    }
    press(&mut app, KeyCode::Char('k'), KeyModifiers::NONE);
    assert_eq!(
        app.keyboard.state.patch(),
        Some("patches_factory/Lead/Lead 0.fxp")
    );
}
