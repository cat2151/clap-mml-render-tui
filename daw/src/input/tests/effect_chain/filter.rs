use super::*;

fn add_list_names(app: &DawApp) -> Vec<String> {
    let catalog = app.effect_plugins.catalog().unwrap();
    app.overlays
        .effect_chain
        .add
        .list
        .iter()
        .map(|&index| catalog.presets()[index].name.clone())
        .collect()
}

#[test]
fn slash_query_filters_the_list_by_role_and_enter_confirms() {
    let (mut app, _cache_rx) = app_with_catalog();
    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);
    assert_eq!(add_list_names(&app).len(), 3);

    press(
        &mut app,
        &[
            KeyCode::Char('/'),
            KeyCode::Char('r'),
            KeyCode::Char('e'),
            KeyCode::Char('v'),
            KeyCode::Enter,
        ],
    );

    assert!(!app.overlays.effect_chain.add.filter_active);
    assert_eq!(add_list_names(&app), vec!["Room"]);
}

#[test]
fn clearing_the_query_and_enter_restores_the_full_list() {
    let (mut app, _cache_rx) = app_with_catalog();
    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);

    press(
        &mut app,
        &[
            KeyCode::Char('/'),
            KeyCode::Char('r'),
            KeyCode::Char('e'),
            KeyCode::Char('v'),
            KeyCode::Backspace,
            KeyCode::Backspace,
            KeyCode::Backspace,
            KeyCode::Enter,
        ],
    );

    assert_eq!(app.overlays.effect_chain.add.query, "");
    assert_eq!(add_list_names(&app).len(), 3);
}

#[test]
fn esc_while_editing_the_query_restores_the_list_from_before_editing() {
    let (mut app, _cache_rx) = app_with_catalog();
    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);
    let before = app.overlays.effect_chain.add.list.clone();

    press(
        &mut app,
        &[
            KeyCode::Char('/'),
            KeyCode::Char('x'),
            KeyCode::Char('y'),
            KeyCode::Char('z'),
            KeyCode::Esc,
        ],
    );

    assert!(!app.overlays.effect_chain.add.filter_active);
    assert_eq!(app.overlays.effect_chain.add.query, "");
    assert_eq!(app.overlays.effect_chain.add.list, before);
}

#[test]
fn typing_j_while_editing_the_query_goes_to_the_textarea_and_the_list_cursor_stays() {
    let (mut app, _cache_rx) = app_with_catalog();
    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);
    app.overlays.effect_chain.add.list_cursor = 0;

    press(&mut app, &[KeyCode::Char('/'), KeyCode::Char('j')]);

    assert_eq!(app.overlays.effect_chain.add.list_cursor, 0);
    assert_eq!(app.overlays.effect_chain.add.query, "j");
}

#[test]
fn an_invalid_regex_query_keeps_the_previous_list_instead_of_emptying_it() {
    let (mut app, _cache_rx) = app_with_catalog();
    press(&mut app, &[KeyCode::Char('x'), KeyCode::Char('a')]);
    let before = app.overlays.effect_chain.add.list.clone();

    press(&mut app, &[KeyCode::Char('/'), KeyCode::Char('(')]);

    assert_eq!(app.overlays.effect_chain.add.query, "(");
    assert_eq!(app.overlays.effect_chain.add.list, before);
}
