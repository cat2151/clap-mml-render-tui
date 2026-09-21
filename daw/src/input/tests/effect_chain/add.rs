use super::*;

#[test]
fn opening_the_add_overlay_lists_categories_and_kinds_and_excludes_reverb_1() {
    let (mut app, _cache_rx) = app_with_catalog();

    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);

    let add = &app.overlays.effect_chain.add;
    assert_eq!(
        add.categories,
        vec!["all", "Distortion / Saturation", "Space / Imaging"]
    );
    assert_eq!(add.kinds, vec!["all", "Amp Simulator", "Delay", "Reverb"]);
    assert_eq!(add.focus, EffectAddPane::List);
    assert_eq!(add.list_cursor, 0);
    let catalog = app.effect_plugins.catalog().unwrap();
    assert!(
        add.list
            .iter()
            .all(|&index| catalog.presets()[index].value != "Reverb 1/Hall"),
        "Reverb 1 の preset は list に出ない: {:?}",
        add.list
    );
}

#[test]
fn h_and_l_move_focus_between_the_three_panes_and_stop_at_the_edges() {
    let (mut app, _cache_rx) = app_with_catalog();
    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);
    assert_eq!(app.overlays.effect_chain.add.focus, EffectAddPane::List);

    press(&mut app, &[KeyCode::Char('h')]);
    assert_eq!(app.overlays.effect_chain.add.focus, EffectAddPane::Kinds);
    press(&mut app, &[KeyCode::Char('h')]);
    assert_eq!(
        app.overlays.effect_chain.add.focus,
        EffectAddPane::Categories
    );
    press(&mut app, &[KeyCode::Char('h')]);
    assert_eq!(
        app.overlays.effect_chain.add.focus,
        EffectAddPane::Categories,
        "端で止まる"
    );

    press(
        &mut app,
        &[KeyCode::Char('l'), KeyCode::Char('l'), KeyCode::Char('l')],
    );
    assert_eq!(
        app.overlays.effect_chain.add.focus,
        EffectAddPane::List,
        "端で止まる"
    );
}

#[test]
fn category_pane_narrows_kinds_and_kind_pane_narrows_the_list() {
    let (mut app, _cache_rx) = app_with_catalog();

    press(
        &mut app,
        &[
            KeyCode::Char('x'),
            KeyCode::Char('a'),
            KeyCode::Char('h'),
            KeyCode::Char('h'),
            KeyCode::Char('j'),
            KeyCode::Char('j'),
        ],
    );

    let add = &app.overlays.effect_chain.add;
    assert_eq!(add.kinds, vec!["all", "Delay", "Reverb"]);
    assert_eq!(add.kind_cursor, 0, "category を動かした直後は all へ戻る");
    assert_eq!(add.list_cursor, 0);
    assert_eq!(add.list.len(), 2);

    press(&mut app, &[KeyCode::Char('l'), KeyCode::Char('j')]);
    let add = &app.overlays.effect_chain.add;
    assert_eq!(add.list.len(), 1);
    let catalog = app.effect_plugins.catalog().unwrap();
    assert_eq!(catalog.presets()[add.list[0]].value, "Delay/Echo");

    press(&mut app, &[KeyCode::Char('l'), KeyCode::Enter]);
    assert!(matches!(app.mode, DawMode::EffectChain));
    assert_eq!(
        app.overlays.effect_chain.chain,
        vec![json!({"Test FX preset": "Delay/Echo"})]
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
