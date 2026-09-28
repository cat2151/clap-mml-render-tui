use super::*;

fn plugins() -> Vec<PatchCatalogEntry> {
    vec![
        entry("Harp.floe-preset", "Floe", None),
        entry("Dexed_01.syx/00 Floe Bell", "Dexed", None),
        entry("Piano.sfz", "Sforzando", None),
        entry("Pads/Warm.fxp", "Surge XT", None),
        entry("Warm Pad.vvp", "Vaporizer2", None),
    ]
}

fn opened_plugins() -> PatchSelect<'static> {
    open_with(plugins(), None, Vec::new())
}

fn shift(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::SHIFT)
}

/// menu を開いて 1 キー選ぶ。
fn choose(select: &mut PatchSelect<'_>, key: KeyEvent) -> PatchSelectAction {
    select.handle_key(press(KeyCode::Char('m')));
    select.handle_key(key)
}

fn query(select: &PatchSelect<'_>) -> String {
    text_input::textarea_value(select.query_textarea())
}

#[test]
fn keys_are_the_first_free_letter_of_each_plugin_name() {
    let mut select = opened_plugins();
    select.handle_key(press(KeyCode::Char('m')));

    let keys: Vec<(char, &str)> = select
        .plugin_menu()
        .expect("menu is open")
        .items()
        .iter()
        .map(|item| (item.key, item.slug.as_str()))
        .collect();
    assert_eq!(
        keys,
        [
            ('d', "dexed"),
            ('f', "floe"),
            ('s', "sforzando"),
            ('u', "surgext"),
            ('v', "vaporizer2"),
        ]
    );
    assert!(select.captures_all_keys());
}

#[test]
fn lowercase_solos_the_plugin_and_closes_the_menu() {
    let mut select = opened_plugins();

    choose(&mut select, press(KeyCode::Char('f')));

    assert!(select.plugin_menu().is_none());
    assert_eq!(query(&select), "plugin:floe");
    // 表示パスに floe を含む Dexed の音色は入らない。
    assert_eq!(filtered(&select), ["Harp.floe-preset"]);
}

#[test]
fn uppercase_mutes_the_plugin_with_or_without_shift_modifier() {
    let mut select = opened_plugins();

    choose(&mut select, shift('D'));
    choose(&mut select, press(KeyCode::Char('U')));

    assert_eq!(query(&select), "-plugin:dexed -plugin:surgext");
    assert_eq!(
        filtered(&select),
        ["Harp.floe-preset", "Piano.sfz", "Warm Pad.vvp"]
    );
}

#[test]
fn the_same_choice_again_removes_it() {
    let mut select = opened_plugins();

    choose(&mut select, press(KeyCode::Char('f')));
    choose(&mut select, press(KeyCode::Char('f')));
    assert_eq!(query(&select), "");

    choose(&mut select, shift('D'));
    choose(&mut select, shift('V'));
    choose(&mut select, shift('D'));
    assert_eq!(query(&select), "-plugin:vaporizer2");
}

#[test]
fn switching_between_solo_and_mute_clears_the_other_side() {
    let mut select = opened_plugins();

    choose(&mut select, press(KeyCode::Char('f')));
    choose(&mut select, shift('D'));
    assert_eq!(query(&select), "-plugin:dexed");

    choose(&mut select, shift('V'));
    choose(&mut select, press(KeyCode::Char('s')));
    assert_eq!(query(&select), "plugin:sforzando");
    assert_eq!(filtered(&select), ["Piano.sfz"]);
}

#[test]
fn solos_stack_and_show_any_of_the_soloed_plugins() {
    let mut select = opened_plugins();

    choose(&mut select, press(KeyCode::Char('f')));
    choose(&mut select, press(KeyCode::Char('v')));

    assert_eq!(query(&select), "plugin:floe plugin:vaporizer2");
    assert_eq!(filtered(&select), ["Harp.floe-preset", "Warm Pad.vvp"]);

    choose(&mut select, press(KeyCode::Char('f')));
    assert_eq!(query(&select), "plugin:vaporizer2");
}

#[test]
fn regex_terms_are_kept_and_combined_with_plugin_terms() {
    let mut select = opened_plugins();
    type_text(&mut select, "warm");
    select.handle_key(press(KeyCode::Enter));

    choose(&mut select, shift('U'));

    assert_eq!(query(&select), "warm -plugin:surgext");
    assert_eq!(filtered(&select), ["Warm Pad.vvp"]);
    // `/` の編集は、menu が書いた条件から始まる。
    select.handle_key(press(KeyCode::Char('/')));
    assert_eq!(query(&select), "warm -plugin:surgext");
}

#[test]
fn escape_and_unknown_keys_leave_the_filter_alone() {
    let mut select = opened_plugins();
    select.handle_key(press(KeyCode::Char('m')));

    // menu に無いキーは閉じずに無視する。Enter で音色を確定しない。
    assert!(matches!(
        select.handle_key(press(KeyCode::Char('z'))),
        PatchSelectAction::Continue
    ));
    assert!(matches!(
        select.handle_key(press(KeyCode::Enter)),
        PatchSelectAction::Continue
    ));
    assert!(select.plugin_menu().is_some());

    select.handle_key(press(KeyCode::Esc));
    assert!(select.plugin_menu().is_none());
    assert_eq!(query(&select), "");
    assert_eq!(filtered(&select).len(), plugins().len());
}

#[test]
fn menu_marks_follow_the_committed_filter() {
    let mut select = opened_plugins();
    choose(&mut select, shift('D'));

    assert_eq!(select.plugin_mode("dexed"), Some(PluginMode::Mute));
    assert_eq!(select.plugin_mode("floe"), None);
}
