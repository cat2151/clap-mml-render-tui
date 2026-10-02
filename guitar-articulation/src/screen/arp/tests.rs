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
        enabled: true,
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
    let mut screen = screen_with_mml("l16cdef");
    assert_eq!(screen.column_count(), 4);

    let action = screen.set_arp(on(ArpPattern::UpDown));

    assert_eq!(action, GuitarArticulationAction::Play(Take::Converted));
    assert_eq!(screen.column_count(), 12);
    assert_eq!(screen.mml(), "l16cdef");
    assert_eq!(*screen.arp(), on(ArpPattern::UpDown));
    let plain = screen.events(Take::Plain).to_vec();
    assert_eq!(
        plain,
        crate::performance_events("l16cdef", &on(ArpPattern::UpDown)).unwrap()
    );
    assert_eq!(
        screen.events(Take::Converted),
        convert(&plain, screen.rules()).as_slice()
    );
}

#[test]
fn the_same_settings_change_nothing_and_do_not_play() {
    let mut screen = screen_with_mml("l16cdef");
    screen.set_arp(on(ArpPattern::UpDown));
    let history_len = screen.history().entries.len();

    let action = screen.set_arp(on(ArpPattern::UpDown));

    assert_eq!(action, GuitarArticulationAction::Continue);
    assert_eq!(screen.history().entries.len(), history_len);
}

#[test]
fn turning_the_arp_off_plays_the_mml_as_written() {
    let mut screen = screen_with_mml("l16cdef");
    screen.set_arp(on(ArpPattern::UpDown));

    screen.set_arp(ArpSettings {
        enabled: false,
        ..on(ArpPattern::UpDown)
    });

    assert_eq!(screen.column_count(), 4);
    assert_eq!(screen.arp().pattern, ArpPattern::UpDown);
}

#[test]
fn a_slide_stays_at_the_same_step_when_the_pattern_changes() {
    let mut screen = screen_with_mml("l16cdef");
    screen.set_arp(on(ArpPattern::UpDown));
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('/')));
    assert_eq!(column_rules(&screen), vec![(2, Rule::Slide)]);

    screen.set_arp(on(ArpPattern::Down));

    assert_eq!(screen.column_count(), 8);
    assert_eq!(column_rules(&screen), vec![(2, Rule::Slide)]);
    assert_eq!(screen.cursor(), 0);
    let anchor = screen.anchor.as_ref().unwrap();
    assert_eq!(anchor.arp, on(ArpPattern::UpDown));
}

#[test]
fn the_history_entry_carries_the_arp_and_applying_it_restores_the_columns() {
    let mut screen = screen_with_mml("l16cdef");
    screen.set_arp(on(ArpPattern::UpDown));
    let entry = screen.history().entries[0].clone();
    assert_eq!(entry.arp, on(ArpPattern::UpDown));
    assert_eq!(entry.mml, "l16cdef");

    screen.set_arp(ArpSettings::default());
    assert_eq!(screen.column_count(), 4);
    screen.apply_history_entry(&entry);

    assert_eq!(*screen.arp(), on(ArpPattern::UpDown));
    assert_eq!(screen.column_count(), 12);
}

#[test]
fn an_entry_saved_without_arp_reads_as_arp_off() {
    let entry: GuitarArticulationHistoryEntry =
        serde_json::from_str(r#"{"mml": "l16cdef", "rules": {}}"#).unwrap();
    assert_eq!(entry.arp, ArpSettings::default());

    let mut screen = screen_with_mml("l16cdef");
    screen.set_arp(on(ArpPattern::UpDown));
    screen.apply_history_entry(&entry);

    assert!(!screen.arp().enabled);
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

#[test]
fn overlay_keys_pick_the_pattern_and_the_octaves() {
    let mut screen = screen_with_overlay();

    let action = screen.handle_key_event(char_key('p'));
    assert_eq!(action, GuitarArticulationAction::Play(Take::Converted));
    assert_eq!(*screen.arp(), on(ArpPattern::UpDown));
    assert_eq!(screen.column_count(), 12);

    let action = screen.handle_key_event(char_key('2'));
    assert_eq!(action, GuitarArticulationAction::Play(Take::Converted));
    assert_eq!(screen.arp().octaves, 2);
    assert_eq!(
        screen.events(Take::Plain),
        crate::performance_events("l16cdef", screen.arp())
            .unwrap()
            .as_slice()
    );

    screen.handle_key_event(char_key('P'));
    assert_eq!(screen.arp().pattern, ArpPattern::DownUp);
    screen.handle_key_event(char_key('t'));
    assert_eq!(screen.arp().pattern, ArpPattern::UpTurn);
    assert!(screen.arp_overlay_open());
}

#[test]
fn cycles_and_turn_stop_at_their_ends_without_playing() {
    let mut screen = screen_with_overlay();
    screen.handle_key_event(char_key('t'));

    for _ in 0..6 {
        screen.handle_key_event(char_key('n'));
    }
    assert_eq!(screen.arp().cycles, 8);
    assert_eq!(
        screen.handle_key_event(char_key('n')),
        GuitarArticulationAction::Continue
    );
    assert_eq!(
        screen.handle_key_event(char_key('N')),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert_eq!(screen.arp().cycles, 7);
    for _ in 0..6 {
        screen.handle_key_event(char_key('N'));
    }
    assert_eq!(screen.arp().cycles, 1);
    assert_eq!(
        screen.handle_key_event(char_key('N')),
        GuitarArticulationAction::Continue
    );

    screen.handle_key_event(char_key('b'));
    assert_eq!(screen.arp().turn, 3);
    assert_eq!(
        screen.handle_key_event(char_key('b')),
        GuitarArticulationAction::Continue
    );
    screen.handle_key_event(char_key('B'));
    screen.handle_key_event(char_key('B'));
    assert_eq!(screen.arp().turn, 1);
    assert_eq!(
        screen.handle_key_event(char_key('B')),
        GuitarArticulationAction::Continue
    );
}

#[test]
fn zero_turns_the_arp_off_and_a_pattern_key_brings_back_the_held_values() {
    let mut screen = screen_with_overlay();
    screen.handle_key_event(char_key('p'));
    screen.handle_key_event(char_key('2'));
    screen.handle_key_event(char_key('n'));

    let action = screen.handle_key_event(char_key('0'));
    assert_eq!(action, GuitarArticulationAction::Play(Take::Converted));
    assert!(!screen.arp().enabled);
    assert_eq!(screen.column_count(), 4);
    assert_eq!(screen.arp().octaves, 2);
    assert_eq!(screen.arp().cycles, 3);
    assert_eq!(
        screen.handle_key_event(char_key('0')),
        GuitarArticulationAction::Continue
    );

    screen.handle_key_event(char_key('u'));
    assert_eq!(
        *screen.arp(),
        ArpSettings {
            enabled: true,
            pattern: ArpPattern::Up,
            octaves: 2,
            cycles: 3,
            ..ArpSettings::default()
        }
    );
    // 4 声部 × 2 オクターブ × 3 周期。
    assert_eq!(screen.column_count(), 24);
}

#[test]
fn space_plays_and_closing_keys_hand_p_back_to_the_screen() {
    for close in [KeyCode::Enter, KeyCode::Esc, KeyCode::Char('z')] {
        let mut screen = screen_with_overlay();
        screen.handle_key_event(char_key('p'));
        assert_eq!(
            screen.handle_key_event(char_key(' ')),
            GuitarArticulationAction::Play(Take::Converted)
        );

        screen.handle_key_event(key(close));
        assert!(!screen.arp_overlay_open(), "{close:?}");
        assert!(screen.arp().enabled, "閉じても設定は残る");

        screen.handle_key_event(char_key('p'));
        assert!(screen.rules().is_on(0, Rule::PinchHarmonic), "{close:?}");
        assert_eq!(screen.arp().pattern, ArpPattern::UpDown);
    }
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
        GuitarArticulationAction::Continue
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
    let mut screen = screen_with_overlay();
    assert_eq!(screen.arp_loop_seconds(), None, "OFF の間は通常の repeat");

    // `l16cdef`（1 step = 0.125 秒）の UpDown 2 周期 = 6 step × 2。
    screen.handle_key_event(char_key('p'));
    let seconds = screen.arp_loop_seconds().unwrap();
    assert!((seconds - 12.0 * 0.125).abs() < 1e-9, "{seconds}");
}
