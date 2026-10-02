use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use crate::{convert, ArpRow, Rule};

fn key(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE)
}

/// `l16cdef` を確定し、`z` で overlay を開いて param pane の `row` を選んだ画面。
fn screen_on_row(row: ArpRow) -> GuitarArticulationScreen {
    let mut screen = GuitarArticulationScreen::default();
    screen.handle_key_event(key('i'));
    for ch in "l16cdef".chars() {
        screen.handle_key_event(key(ch));
    }
    screen.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    screen.handle_key_event(key('z'));
    let index = screen.arp_rows().iter().position(|shown| *shown == row);
    for _ in 0..index.expect("行が出ていない") {
        screen.handle_key_event(key('j'));
    }
    assert_eq!(screen.arp_rows()[screen.arp_row_index().unwrap()], row);
    screen
}

fn humanize_pair(screen: &GuitarArticulationScreen) -> (bool, bool) {
    (
        screen.rules().is_row_on(RowRule::Humanize),
        screen.rules().is_row_on(RowRule::HumanizeRelease),
    )
}

/// (エコノミーピッキング, 自動ハンマリング, 自動ハンマリングの選び方)。
fn picking_triple(screen: &GuitarArticulationScreen) -> (bool, bool, AutoPick) {
    (
        screen.rules().is_row_on(RowRule::EconomyPicking),
        screen.rules().is_row_on(RowRule::AutoHammerPull),
        screen.rules().auto_pick(),
    )
}

#[test]
fn the_humanize_row_walks_the_four_pairs_and_stops_at_the_ends() {
    let mut screen = screen_on_row(ArpRow::Humanize);
    assert_eq!(screen.arp_humanize_label(), "汚しなし");
    assert_eq!(
        screen.handle_key_event(key('h')),
        GuitarArticulationAction::Continue
    );

    let mut walked = Vec::new();
    for _ in 0..4 {
        screen.handle_key_event(key('l'));
        walked.push((screen.arp_humanize_label(), humanize_pair(&screen)));
    }
    assert_eq!(
        walked,
        [
            ("汚し&汚しrelease", (true, true)),
            ("汚し", (true, false)),
            ("汚しrelease", (false, true)),
            ("汚しrelease", (false, true)),
        ]
    );
    for _ in 0..4 {
        screen.handle_key_event(key('h'));
    }
    assert_eq!(humanize_pair(&screen), (false, false));
}

#[test]
fn the_picking_row_walks_the_four_ways_and_keeps_them_exclusive() {
    let mut screen = screen_on_row(ArpRow::Picking);
    assert_eq!(screen.arp_picking_label(), "ピッキング");
    assert_eq!(
        screen.handle_key_event(key('h')),
        GuitarArticulationAction::Continue
    );

    let mut walked = Vec::new();
    for _ in 0..4 {
        screen.handle_key_event(key('l'));
        walked.push((screen.arp_picking_label(), picking_triple(&screen)));
    }
    assert_eq!(
        walked,
        [
            ("エコ", (true, false, AutoPick::Run)),
            ("オートプリング1", (false, true, AutoPick::Run)),
            ("オートプリング2", (false, true, AutoPick::Accent)),
            ("オートプリング2", (false, true, AutoPick::Accent)),
        ]
    );

    let mut back = Vec::new();
    for _ in 0..4 {
        screen.handle_key_event(key('h'));
        back.push(picking_triple(&screen));
    }
    assert_eq!(
        back,
        [
            (false, true, AutoPick::Run),
            (true, false, AutoPick::Run),
            (false, false, AutoPick::Run),
            (false, false, AutoPick::Run),
        ]
    );
}

#[test]
fn the_picking_row_starts_from_the_main_screen_value() {
    let mut screen = GuitarArticulationScreen::default();
    screen.handle_key_event(key('i'));
    for ch in "l16cdef".chars() {
        screen.handle_key_event(key(ch));
    }
    screen.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    // メイン画面の `s` を 2 回で on2。
    screen.handle_key_event(key('s'));
    screen.handle_key_event(key('s'));
    screen.handle_key_event(key('z'));
    assert_eq!(screen.arp_picking_label(), "オートプリング2");
}

#[test]
fn changing_a_row_plays_the_converted_take_without_column_rules_and_adds_no_history() {
    let mut screen = screen_on_row(ArpRow::Humanize);
    let history = screen.history().entries.len();

    assert_eq!(
        screen.handle_key_event(key('l')),
        GuitarArticulationAction::Play(Take::Converted)
    );
    screen.handle_key_event(key('j'));
    assert_eq!(
        screen.handle_key_event(key('l')),
        GuitarArticulationAction::Play(Take::Converted)
    );

    assert_eq!(screen.history().entries.len(), history);
    assert_eq!(
        screen.events(Take::Converted),
        convert(
            screen.events(Take::Plain),
            &screen.rules().without_column_rules()
        )
        .as_slice()
    );
}

#[test]
fn closing_after_a_change_keeps_the_rules_and_records_one_history_entry() {
    let mut screen = screen_on_row(ArpRow::Picking);
    let history = screen.history().entries.len();
    screen.handle_key_event(key('l'));
    screen.handle_key_event(key('k'));
    screen.handle_key_event(key('l'));

    screen.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert!(!screen.arp_overlay_open());
    assert!(screen.rules().is_row_on(RowRule::EconomyPicking));
    assert!(screen.rules().is_row_on(RowRule::Humanize));
    assert_eq!(screen.history().entries.len(), history + 1);
    assert_eq!(screen.history().entries[0].rules, *screen.rules());
    assert!(screen.take_unsaved_history());
    assert!(!screen.take_unsaved_history());
}

#[test]
fn closing_without_a_net_change_records_nothing() {
    let mut screen = screen_on_row(ArpRow::Humanize);
    let history = screen.history().entries.len();
    screen.handle_key_event(key('l'));
    screen.handle_key_event(key('h'));

    screen.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(screen.history().entries.len(), history);
    assert!(!screen.take_unsaved_history());
}

#[test]
fn the_rows_keep_the_main_column_rules() {
    let mut screen = GuitarArticulationScreen::default();
    screen.handle_key_event(key('i'));
    for ch in "l16cdef".chars() {
        screen.handle_key_event(key(ch));
    }
    screen.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    screen.rules.toggle(2, Rule::HammerPull);
    screen.handle_key_event(key('z'));
    let picking = screen
        .arp_rows()
        .iter()
        .position(|row| *row == ArpRow::Picking);
    for _ in 0..picking.unwrap() {
        screen.handle_key_event(key('j'));
    }
    screen.handle_key_event(key('l'));
    screen.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert!(screen.rules().is_on(2, Rule::HammerPull));
    assert!(screen.rules().is_row_on(RowRule::EconomyPicking));
}

#[test]
fn the_accent_row_walks_top_bottom_both_plays_and_stops_at_the_ends() {
    let mut screen = screen_on_row(ArpRow::Accent);
    let history = screen.history().entries.len();
    assert_eq!(screen.rules().accent_pattern(), AccentPattern::Top);
    assert_eq!(
        screen.handle_key_event(key('h')),
        GuitarArticulationAction::Continue
    );

    let mut walked = Vec::new();
    for _ in 0..3 {
        walked.push((
            screen.handle_key_event(key('l')),
            screen.rules().accent_pattern(),
        ));
    }
    let play = || GuitarArticulationAction::Play(Take::Converted);
    assert_eq!(
        walked,
        [
            (play(), AccentPattern::Bottom),
            (play(), AccentPattern::Both),
            (GuitarArticulationAction::Continue, AccentPattern::Both),
        ]
    );
    assert_eq!(screen.history().entries.len(), history);

    screen.handle_key_event(key('h'));
    assert_eq!(screen.rules().accent_pattern(), AccentPattern::Bottom);
    screen.handle_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(screen.rules().accent_pattern(), AccentPattern::Bottom);
    assert_eq!(screen.history().entries.len(), history + 1);
    assert!(screen.take_unsaved_history());
}

#[test]
fn the_accent_row_starts_from_the_main_screen_value() {
    let mut screen = GuitarArticulationScreen::default();
    screen.handle_key_event(key('i'));
    for ch in "l16cdef".chars() {
        screen.handle_key_event(key(ch));
    }
    screen.handle_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    screen.handle_key_event(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT));
    screen.handle_key_event(key('z'));
    assert_eq!(screen.rules().accent_pattern(), AccentPattern::Bottom);
    for _ in 0..screen.arp_rows().len() {
        screen.handle_key_event(key('j'));
    }
    screen.handle_key_event(key('l'));
    assert_eq!(screen.rules().accent_pattern(), AccentPattern::Both);
}
