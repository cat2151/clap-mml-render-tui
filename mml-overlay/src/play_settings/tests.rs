use super::*;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

fn settled(select: PlaySettingsAction) -> PlaySettings {
    match select {
        PlaySettingsAction::Confirm(settings) | PlaySettingsAction::Cancel(settings) => settings,
        PlaySettingsAction::Continue => panic!("まだ閉じていない"),
    }
}

#[test]
fn space_toggles_the_selected_item() {
    let mut select = PlaySettingsSelect::open(PlaySettings::default());

    select.handle_key(press(KeyCode::Char(' ')));

    assert!(select.settings().repeat);
}

/// ←→ は「値を選ぶ」列ではなく checkbox なので、どちらも同じ切り替えになる。
#[test]
fn left_and_right_toggle_too() {
    let mut select = PlaySettingsSelect::open(PlaySettings::default());

    select.handle_key(press(KeyCode::Right));
    assert!(select.settings().repeat);

    select.handle_key(press(KeyCode::Left));
    assert!(!select.settings().repeat);
}

#[test]
fn the_cursor_walks_the_three_items_and_stops_at_both_ends() {
    let mut select = PlaySettingsSelect::open(PlaySettings::default());

    for _ in 0..5 {
        select.handle_key(press(KeyCode::Down));
    }
    assert_eq!(select.cursor(), 2);
    select.handle_key(press(KeyCode::Char(' ')));
    assert!(select.settings().filters.velocity);

    for _ in 0..5 {
        select.handle_key(press(KeyCode::Up));
    }
    assert_eq!(select.cursor(), 0);
}

#[test]
fn enter_confirms_the_edited_values() {
    let mut select = PlaySettingsSelect::open(PlaySettings::default());
    select.handle_key(press(KeyCode::Char(' ')));
    select.handle_key(press(KeyCode::Down));
    select.handle_key(press(KeyCode::Char(' ')));

    let settings = settled(select.handle_key(press(KeyCode::Enter)));

    assert_eq!(
        settings,
        PlaySettings {
            repeat: true,
            filters: FilterSettings {
                modulation: true,
                velocity: false,
            },
        }
    );
}

/// 複数の項目を触ってからの取り消しでも、開いた時点の値へ丸ごと戻る。
#[test]
fn esc_rolls_back_every_item_to_the_value_it_opened_with() {
    let opened_with = PlaySettings {
        repeat: true,
        filters: FilterSettings {
            modulation: false,
            velocity: true,
        },
    };
    let mut select = PlaySettingsSelect::open(opened_with);
    for _ in 0..3 {
        select.handle_key(press(KeyCode::Char(' ')));
        select.handle_key(press(KeyCode::Down));
    }

    assert_eq!(settled(select.handle_key(press(KeyCode::Esc))), opened_with);
}

#[test]
fn q_cancels_as_well() {
    let mut select = PlaySettingsSelect::open(PlaySettings::default());
    select.handle_key(press(KeyCode::Char(' ')));

    assert_eq!(
        settled(select.handle_key(press(KeyCode::Char('q')))),
        PlaySettings::default()
    );
}

/// 開くキーをもう一度押したら取り消して閉じる。
#[test]
fn ctrl_l_cancels_as_well() {
    let mut select = PlaySettingsSelect::open(PlaySettings::default());
    select.handle_key(press(KeyCode::Char(' ')));

    assert_eq!(
        settled(select.handle_key(ctrl(KeyCode::Char('l')))),
        PlaySettings::default()
    );
}

#[test]
fn ctrl_l_is_the_trigger_and_a_bare_l_is_not() {
    assert!(is_play_settings_trigger(ctrl(KeyCode::Char('l'))));
    assert!(!is_play_settings_trigger(press(KeyCode::Char('l'))));
}

#[test]
fn every_item_knows_its_own_on_off() {
    let settings = PlaySettings {
        repeat: false,
        filters: FilterSettings {
            modulation: true,
            velocity: false,
        },
    };

    let on = PlaySettingsItem::ALL.map(|item| item.is_on(&settings));

    assert_eq!(on, [false, true, false]);
}
