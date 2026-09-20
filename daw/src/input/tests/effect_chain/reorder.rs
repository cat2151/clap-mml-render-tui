use super::*;

#[test]
fn b_toggles_bypass_on_the_stage_under_the_cursor_and_enter_writes_it() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("daw_cache");
    let (mut app, _cache_rx) = app_with_catalog();
    app.editor.data[2][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"Test FX preset":"Hall"}]}"#.to_string();

    press(
        &mut app,
        &[KeyCode::Char('x'), KeyCode::Char('b'), KeyCode::Enter],
    );

    assert_eq!(
        init_json(&app),
        json!({
            "Surge XT patch": "Pads/Pad 1.fxp",
            "effects after instrument": [{"Test FX preset": "Hall", "bypass": true}],
        })
    );

    press(
        &mut app,
        &[KeyCode::Char('x'), KeyCode::Char('b'), KeyCode::Enter],
    );

    assert_eq!(
        init_json(&app),
        json!({
            "Surge XT patch": "Pads/Pad 1.fxp",
            "effects after instrument": [{"Test FX preset": "Hall"}],
        })
    );
}

#[test]
fn alt_down_swaps_the_stage_with_the_next_one_and_the_cursor_follows() {
    let (mut app, _cache_rx) = app_with_catalog();
    app.editor.data[2][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"Test FX preset":"Hall"},{"Test FX preset":"Room"}]}"#.to_string();
    app.handle_normal(KeyCode::Char('x'));

    app.handle_effect_chain(KeyEvent::new(KeyCode::Down, KeyModifiers::ALT));

    assert_eq!(
        app.overlays.effect_chain.chain,
        vec![
            json!({"Test FX preset": "Room"}),
            json!({"Test FX preset": "Hall"}),
        ]
    );
    assert_eq!(app.overlays.effect_chain.cursor, 1);

    // 末尾での Alt+Down は無変化。
    app.handle_effect_chain(KeyEvent::new(KeyCode::Down, KeyModifiers::ALT));

    assert_eq!(
        app.overlays.effect_chain.chain,
        vec![
            json!({"Test FX preset": "Room"}),
            json!({"Test FX preset": "Hall"}),
        ]
    );
    assert_eq!(app.overlays.effect_chain.cursor, 1);
}

#[test]
fn page_down_home_end_move_the_cursor_across_three_stages() {
    let (mut app, _cache_rx) = app_with_catalog();
    app.editor.data[2][0] = r#"{"Surge XT patch":"Pads/Pad 1.fxp","effects after instrument":[{"Test FX preset":"Hall"},{"Test FX preset":"Room"},{"Test FX preset":"Hall"}]}"#.to_string();
    app.handle_normal(KeyCode::Char('x'));

    app.handle_effect_chain(KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE));
    assert_eq!(app.overlays.effect_chain.cursor, 2);

    app.handle_effect_chain(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
    assert_eq!(app.overlays.effect_chain.cursor, 0);

    app.handle_effect_chain(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
    assert_eq!(app.overlays.effect_chain.cursor, 2);
}
