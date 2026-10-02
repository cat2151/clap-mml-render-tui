//! chord 素材の BPM と音価の行。

use super::*;
use crate::ArpRate;

/// `mml` を確定し、`z` で overlay を開いて Up で ON にした画面。
fn chord_screen(mml: &str) -> GuitarArticulationScreen {
    let mut screen = screen_with_mml(mml);
    screen.handle_key_event(key(KeyCode::Char('z')));
    screen.set_arp(on(ArpPattern::Up));
    screen
}

#[test]
fn the_material_kind_follows_the_committed_mml() {
    assert!(chord_screen("Am7").material_from_chord());
    assert!(!chord_screen("l16cdef").material_from_chord());
}

#[test]
fn the_chord_arp_loop_is_the_step_count_times_the_bpm_step() {
    let mut screen = chord_screen("Am7");
    // Am7 は 4 声部、Up 1 周期 = 4 step、120 BPM の 16 分 = 0.125 秒。
    assert_eq!(screen.column_count(), 4);
    let seconds = screen.arp_loop_seconds().unwrap();
    assert!((seconds - 4.0 * 0.125).abs() < 1e-9, "{seconds}");

    select_row(&mut screen, ArpRow::Rate);
    screen.handle_key_event(char_key('h'));
    assert_eq!(screen.arp().rate, ArpRate::EighthTriplet);
    let seconds = screen.arp_loop_seconds().unwrap();
    assert!((seconds - 4.0 * 0.5 / 3.0).abs() < 1e-9, "{seconds}");
}

#[test]
fn the_bpm_moves_by_five_with_h_l_and_by_one_with_shift() {
    let mut screen = chord_screen("C");
    select_row(&mut screen, ArpRow::Bpm);
    assert_eq!(screen.arp().bpm, 120);
    assert_eq!(
        screen.handle_key_event(char_key('l')),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert_eq!(screen.arp().bpm, 125);
    screen.handle_key_event(char_key('h'));
    screen.handle_key_event(char_key('h'));
    assert_eq!(screen.arp().bpm, 115);
    screen.handle_key_event(char_key('L'));
    assert_eq!(screen.arp().bpm, 116);
    screen.handle_key_event(char_key('H'));
    screen.handle_key_event(char_key('H'));
    assert_eq!(screen.arp().bpm, 114);

    for _ in 0..40 {
        screen.handle_key_event(char_key('l'));
    }
    assert_eq!(screen.arp().bpm, 240);
    assert_eq!(
        screen.handle_key_event(char_key('L')),
        GuitarArticulationAction::Continue
    );
    for _ in 0..60 {
        screen.handle_key_event(char_key('h'));
    }
    assert_eq!(screen.arp().bpm, 40);
}

#[test]
fn the_rate_walks_the_rates_and_stops_at_its_ends() {
    let mut screen = chord_screen("C");
    select_row(&mut screen, ArpRow::Rate);
    let mut walked = Vec::new();
    for _ in 0..3 {
        screen.handle_key_event(char_key('l'));
        walked.push(screen.arp().rate);
    }
    assert_eq!(
        walked,
        [
            ArpRate::SixteenthTriplet,
            ArpRate::ThirtySecond,
            ArpRate::ThirtySecond
        ]
    );
    for _ in 0..5 {
        screen.handle_key_event(char_key('h'));
    }
    assert_eq!(screen.arp().rate, ArpRate::Eighth);
}

#[test]
fn applying_a_history_entry_restores_the_material_kind() {
    let mut screen = chord_screen("Am7");
    let chord_entry = screen.history_entry();
    let mml_screen = chord_screen("l16cdef");
    screen.apply_history_entry(&mml_screen.history_entry());
    assert!(!screen.material_from_chord());
    screen.apply_history_entry(&chord_entry);
    assert!(screen.material_from_chord());
}
