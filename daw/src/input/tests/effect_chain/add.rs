use super::*;

#[test]
fn opening_the_add_overlay_lists_roles_and_excludes_reverb_1() {
    let (mut app, _cache_rx) = app_with_catalog();

    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);

    let add = &app.overlays.effect_chain.add;
    assert_eq!(add.roles, vec!["all", "Amp Simulator", "Delay", "Reverb 2"]);
    assert_eq!(add.focus, EffectAddPane::List);
    assert_eq!(add.list_cursor, 0);
    let catalog = app.effect_plugins.catalog().unwrap();
    assert!(
        add.list
            .iter()
            .all(|&index| catalog.presets()[index].value != "Hall"),
        "Reverb 1 の preset は list に出ない: {:?}",
        add.list
    );
}

#[test]
fn role_pane_then_list_pane_adds_a_preset_of_the_selected_role() {
    let (mut app, _cache_rx) = app_with_catalog();

    press(
        &mut app,
        &[
            KeyCode::Char('x'),
            KeyCode::Char('a'),
            KeyCode::Char('h'),
            KeyCode::Char('j'),
        ],
    );
    assert_eq!(app.overlays.effect_chain.add.list.len(), 1);

    press(&mut app, &[KeyCode::Char('l'), KeyCode::Enter]);

    assert!(matches!(app.mode, DawMode::EffectChain));
    assert_eq!(
        app.overlays.effect_chain.chain,
        vec![json!({"Test Amp preset": "Clean"})]
    );
}

#[test]
fn enter_on_an_empty_list_does_nothing() {
    let (mut app, _cache_rx) = app_with_catalog();

    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);
    app.overlays.effect_chain.add.list.clear();
    app.overlays.effect_chain.add.list_cursor = 0;

    press(&mut app, &[KeyCode::Enter]);

    assert!(matches!(app.mode, DawMode::EffectChainAdd));
    assert!(app.overlays.effect_chain.chain.is_empty());
}
