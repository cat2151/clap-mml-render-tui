use cmrt_arpeggiator::ArpPattern;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use crate::history::GuitarArticulationHistoryEntry;
use crate::{convert, Rule};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// `i` → 文字 → `Enter` で MML を確定した画面。
fn screen_with_mml(mml: &str) -> GuitarArticulationScreen {
    let mut screen = GuitarArticulationScreen::default();
    screen.handle_key_event(key(KeyCode::Char('i')));
    for ch in mml.chars() {
        screen.handle_key_event(key(KeyCode::Char(ch)));
    }
    screen.handle_key_event(key(KeyCode::Enter));
    screen
}

fn on(pattern: ArpPattern) -> ArpSettings {
    ArpSettings {
        pattern,
        ..ArpSettings::default()
    }
}

/// 列ごとのルールが ON の (列, ルール) を全部。
fn column_rules(screen: &GuitarArticulationScreen) -> Vec<(usize, Rule)> {
    let mut on = Vec::new();
    for column in 0..screen.column_count() {
        for rule in crate::ui::RULE_ROWS.map(|rule_row| rule_row.rule) {
            if screen.rules().is_on(column, rule) {
                on.push((column, rule));
            }
        }
    }
    on
}

#[test]
fn setting_an_arp_rebuilds_the_columns_and_keeps_the_mml() {
    let mut screen = screen_with_overlay();
    assert_eq!(screen.column_count(), 4);

    let action = screen.set_arp(on(ArpPattern::UpDown));

    assert_eq!(action, GuitarArticulationAction::Play(Take::Converted));
    // 上り 4 + 下り 2。
    assert_eq!(screen.column_count(), 6);
    assert_eq!(screen.mml(), "l16cdef");
    assert_eq!(*screen.arp(), on(ArpPattern::UpDown));
    let plain = screen.events(Take::Plain).to_vec();
    assert_eq!(
        plain,
        crate::performance_events("l16cdef", Some(&on(ArpPattern::UpDown))).unwrap()
    );
    assert_eq!(
        screen.events(Take::Converted),
        convert(&plain, screen.rules()).as_slice()
    );
}

#[test]
fn the_same_settings_change_nothing_and_do_not_play() {
    let mut screen = screen_with_overlay();
    screen.set_arp(on(ArpPattern::UpDown));
    let history_len = screen.history().entries.len();

    let action = screen.set_arp(on(ArpPattern::UpDown));

    assert_eq!(action, GuitarArticulationAction::Continue);
    assert_eq!(screen.history().entries.len(), history_len);
}

#[test]
fn the_arp_applies_only_while_the_overlay_is_open_and_keeps_its_values() {
    let mut screen = screen_with_mml("l16cdef");
    assert_eq!(screen.applied_arp(), None);
    screen.handle_key_event(key(KeyCode::Char('z')));
    screen.set_arp(on(ArpPattern::UpDown));
    assert_eq!(screen.applied_arp(), Some(&on(ArpPattern::UpDown)));

    screen.handle_key_event(key(KeyCode::Esc));

    assert_eq!(screen.applied_arp(), None);
    assert_eq!(screen.column_count(), 4);
    assert_eq!(
        screen.events(Take::Plain),
        crate::performance_events("l16cdef", None)
            .unwrap()
            .as_slice()
    );
    assert_eq!(screen.arp().pattern, ArpPattern::UpDown);
    screen.handle_key_event(key(KeyCode::Char('z')));
    assert_eq!(screen.column_count(), 6);
}

#[test]
fn the_overlay_keeps_the_main_column_rules_and_cursor_but_does_not_apply_them() {
    let mut screen =
        screen_with_mml("l16cdef").with_arp_materials(vec!["l16cdef".into(), "Am7".into()]);
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('/')));
    assert_eq!(column_rules(&screen), vec![(2, Rule::Slide)]);
    let rules = screen.rules().clone();
    let anchor = screen.anchor.clone();

    screen.handle_key_event(key(KeyCode::Char('z')));
    screen.set_arp(on(ArpPattern::UpDown));
    screen.handle_key_event(key(KeyCode::Char('l')));
    assert_eq!(screen.arp_material(), "Am7");

    assert_eq!(*screen.rules(), rules, "列ルールは持ったまま");
    assert_eq!(screen.cursor(), 2);
    let plain = screen.events(Take::Plain).to_vec();
    assert_eq!(
        plain,
        crate::material_performance("Am7", Some(&on(ArpPattern::UpDown)))
            .unwrap()
            .events
    );
    assert_eq!(
        screen.events(Take::Converted),
        convert(&plain, &rules.without_column_rules()).as_slice(),
        "overlay では列ルールを当てない"
    );

    screen.handle_key_event(key(KeyCode::Esc));
    assert_eq!(*screen.rules(), rules);
    assert_eq!(column_rules(&screen), vec![(2, Rule::Slide)]);
    assert_eq!(screen.cursor(), 2);
    assert_eq!(screen.anchor, anchor);
    assert_eq!(screen.mml(), "l16cdef");
    let plain = crate::performance_events("l16cdef", None).unwrap();
    assert_eq!(screen.events(Take::Plain), plain.as_slice());
    assert_eq!(
        screen.events(Take::Converted),
        convert(&plain, &rules).as_slice()
    );
}

#[test]
fn opening_changing_and_closing_the_overlay_add_no_history() {
    let mut screen = screen_with_mml("l16cdef").with_arp_materials(vec!["Am7".into()]);
    let history = screen.history().clone();

    screen.handle_key_event(key(KeyCode::Char('z')));
    screen.set_arp(on(ArpPattern::UpDown));
    screen.handle_key_event(key(KeyCode::Char('l')));
    assert_eq!(screen.arp_material(), "Am7");
    screen.handle_key_event(key(KeyCode::Esc));

    assert_eq!(*screen.history(), history);
}

#[test]
fn an_old_entry_with_an_arp_field_restores_only_the_main_screen() {
    let entry: GuitarArticulationHistoryEntry =
        serde_json::from_str(r#"{"mml": "l16cdef", "rules": {}, "arp": {"pattern": "UpDown"}}"#)
            .unwrap();

    let mut screen = GuitarArticulationScreen::default();
    screen.apply_history_entry(&entry);

    assert_eq!(screen.mml(), "l16cdef");
    assert_eq!(*screen.arp(), ArpSettings::default());
    assert_eq!(screen.column_count(), 4);
}

fn char_key(ch: char) -> KeyEvent {
    let modifiers = if ch.is_ascii_uppercase() {
        KeyModifiers::SHIFT
    } else {
        KeyModifiers::NONE
    };
    KeyEvent::new(KeyCode::Char(ch), modifiers)
}

/// `l16cdef` を確定し、`z` で overlay を開いた画面。
fn screen_with_overlay() -> GuitarArticulationScreen {
    let mut screen = screen_with_mml("l16cdef");
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('z'))),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert!(screen.arp_overlay_open());
    screen
}

/// param pane で今選んでいる行。
fn selected_row(screen: &GuitarArticulationScreen) -> ArpRow {
    screen.arp_rows()[screen.arp_row_index().unwrap()]
}

/// `j` / `k` で param pane の `row` を選ぶ。
fn select_row(screen: &mut GuitarArticulationScreen, row: ArpRow) {
    for _ in 0..ALL_ROWS.len() {
        screen.handle_key_event(char_key('k'));
    }
    while selected_row(screen) != row {
        let before = screen.arp_row_index();
        screen.handle_key_event(char_key('j'));
        assert_ne!(screen.arp_row_index(), before, "{row:?} が出ていない");
    }
}

#[test]
fn space_plays_and_closing_keys_hand_p_back_to_the_screen() {
    for close in [KeyCode::Enter, KeyCode::Esc, KeyCode::Char('z')] {
        let mut screen = screen_with_overlay();
        screen.set_arp(on(ArpPattern::UpDown));
        assert_eq!(
            screen.handle_key_event(char_key(' ')),
            GuitarArticulationAction::Play(Take::Converted)
        );

        screen.handle_key_event(key(close));
        assert!(!screen.arp_overlay_open(), "{close:?}");
        assert_eq!(screen.column_count(), 4, "閉じると素材のまま");

        screen.handle_key_event(char_key('p'));
        assert!(screen.rules().is_on(0, Rule::PinchHarmonic), "{close:?}");
        assert_eq!(screen.arp().pattern, ArpPattern::UpDown);
    }
}

#[test]
fn the_old_direct_keys_do_nothing() {
    let mut screen = screen_with_overlay();
    let before = *screen.arp();
    for ch in [
        'u', 'd', 'p', 'P', '1', '2', '<', '>', 'b', 'B', '-', '=', 'r', 'R', '0',
    ] {
        assert_eq!(
            screen.handle_key_event(char_key(ch)),
            GuitarArticulationAction::Continue,
            "{ch}"
        );
    }
    assert_eq!(*screen.arp(), before);
    assert!(screen.arp_overlay_open());
}

#[test]
fn question_opens_the_overlay_help_and_only_closing_keys_work_there() {
    let mut screen = screen_with_overlay();
    screen.handle_key_event(char_key('?'));
    assert!(screen.arp_help_open());
    assert!(!screen.help_open(), "画面のヘルプではない");

    assert_j_does_not_move(&mut screen);
    assert_eq!(
        screen.handle_key_event(char_key('l')),
        GuitarArticulationAction::Continue
    );
    assert_eq!(*screen.arp(), ArpSettings::default());

    screen.handle_key_event(key(KeyCode::Esc));
    assert!(!screen.arp_help_open());
    assert!(screen.arp_overlay_open(), "Esc はヘルプだけを閉じる");
    screen.handle_key_event(char_key('?'));
    screen.handle_key_event(char_key('?'));
    assert!(!screen.arp_help_open());
}

/// ヘルプ中に `j` を押しても行が動かないことを確かめる。
fn assert_j_does_not_move(screen: &mut GuitarArticulationScreen) {
    let before = screen.arp_row_index();
    screen.handle_key_event(char_key('j'));
    assert_eq!(screen.arp_row_index(), before);
}

#[test]
fn z_while_editing_the_mml_is_typed_into_the_input() {
    let mut screen = screen_with_mml("l16cdef");
    screen.handle_key_event(key(KeyCode::Char('i')));
    screen.handle_key_event(key(KeyCode::Char('z')));

    assert!(!screen.arp_overlay_open());
    assert!(screen.input_open());
    let input = screen.input().unwrap();
    assert!(input.lines()[0].ends_with('z'), "{:?}", input.lines());
}

#[test]
fn the_overlay_repeats_while_open_and_closing_stops_unless_shift_r_is_on() {
    let mut screen = screen_with_overlay();
    assert!(screen.repeat());
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Esc)),
        GuitarArticulationAction::StopRepeat
    );
    assert!(!screen.repeat());

    screen.handle_key_event(char_key('R'));
    screen.handle_key_event(key(KeyCode::Char('z')));
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Esc)),
        GuitarArticulationAction::Play(Take::Converted),
        "素材のままの演奏で repeat し直す"
    );
    assert!(screen.repeat(), "Shift+R の ON は閉じても残る");
}

#[test]
fn opening_the_overlay_without_mml_does_not_play() {
    let mut screen = GuitarArticulationScreen::default();
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('z'))),
        GuitarArticulationAction::Continue
    );
    assert!(screen.arp_overlay_open());
}

#[test]
fn the_arp_loop_is_the_step_count_times_one_step() {
    let mut screen = screen_with_mml("l16cdef");
    assert_eq!(
        screen.arp_loop_seconds(),
        None,
        "閉じている間は通常の repeat"
    );

    screen.handle_key_event(key(KeyCode::Char('z')));
    // `l16cdef`（1 step = 0.125 秒）の Up 1 周期 = 4 step、UpDown 1 周期 = 6 step。
    let seconds = screen.arp_loop_seconds().unwrap();
    assert!((seconds - 4.0 * 0.125).abs() < 1e-9, "{seconds}");
    screen.set_arp(on(ArpPattern::UpDown));
    let seconds = screen.arp_loop_seconds().unwrap();
    assert!((seconds - 6.0 * 0.125).abs() < 1e-9, "{seconds}");
}

mod chord_tempo;
mod param_keys;

#[test]
fn a_changed_arp_is_marked_for_the_settings_file_and_the_same_one_is_not() {
    let mut screen = screen_with_overlay();
    screen.take_unsaved_settings();

    screen.set_arp(on(ArpPattern::UpDown));
    assert!(screen.take_unsaved_settings());
    assert_eq!(screen.settings().arp, on(ArpPattern::UpDown));

    screen.set_arp(on(ArpPattern::UpDown));
    assert!(!screen.take_unsaved_settings());
}

#[test]
fn a_restored_material_and_arp_are_where_the_overlay_starts() {
    let mut screen = GuitarArticulationScreen::default()
        .with_arp_material("l16gfed".to_string())
        .with_arp(on(ArpPattern::Down));

    screen.handle_key_event(key(KeyCode::Char('z')));

    assert_eq!(screen.sounding_mml(), "l16gfed");
    assert_eq!(screen.applied_arp(), Some(&on(ArpPattern::Down)));
    assert_eq!(
        screen.events(Take::Plain),
        crate::performance_events("l16gfed", Some(&on(ArpPattern::Down)))
            .unwrap()
            .as_slice()
    );
    assert!(!screen.take_unsaved_settings());
}
