//! repeat（`Shift+R`・アルペジエーター overlay）の演奏を、どの周期で `play_line` へ積むか。

use super::guitar_articulation::{app_with_mml, plain, wait_until};
use super::*;

/// o3 の e（`MML` の頭の音）。
const FIRST_PITCH: u8 = 40;

fn shift(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::SHIFT)
}

fn note_on_seconds(sink: &cmrt_mml_overlay::RecordingSink, pitch: u8) -> Vec<f64> {
    sink.timeline_events()
        .iter()
        .filter(|event| event.message[0] & 0xF0 == 0x90 && event.message[1] == pitch)
        .filter(|event| event.message[2] != 0)
        .map(|event| event.timeline_seconds)
        .collect()
}

#[test]
fn a_short_note_repeats_every_half_second() {
    let (mut app, sink) = app_with_mml();
    // 1 音モードで、頭の 8 分音符（0.25 秒）だけを繰り返す。
    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('n')));
    app.handle_guitar_articulation_key_event(shift('R'));
    wait_until("2 周目が積まれる", || {
        note_on_seconds(&sink, FIRST_PITCH).len() >= 2
    });
    let on = note_on_seconds(&sink, FIRST_PITCH);
    assert!((on[1] - on[0] - 0.5).abs() < 1e-6, "{on:?}");
}

#[test]
fn turning_the_repeat_off_stops_the_take() {
    let (mut app, sink) = app_with_mml();
    app.handle_guitar_articulation_key_event(shift('R'));
    wait_until("積まれる", || sink.timelines() == 1);
    let stops = sink.stops();

    app.handle_guitar_articulation_key_event(shift('R'));

    wait_until("止まる", || sink.stops() > stops);
}

#[test]
fn without_the_repeat_the_take_is_played_once() {
    let (mut app, sink) = app_with_mml();
    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('n')));
    app.handle_guitar_articulation_key_event(plain(KeyCode::Char(' ')));
    // 繰り返すなら先の周まで一度に積むので、1 周目が見えた時点で 2 周目も見える。
    wait_until("積まれる", || {
        !note_on_seconds(&sink, FIRST_PITCH).is_empty()
    });
    assert_eq!(note_on_seconds(&sink, FIRST_PITCH).len(), 1);
}

#[test]
fn the_arp_overlay_repeats_the_arpeggio_without_a_gap() {
    let (mut app, sink) = app_with_mml();
    // `o3 l8 e f+ g` の Up 1 周 = 3 step × 0.25 秒。
    // 最後の列を H/P にすると、末尾に KS の戻しが付いて最後のイベントが 1 周より後ろになる。
    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('l')));
    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('l')));
    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('a')));
    let last_event = app
        .guitar_articulation
        .events(crate::tui::guitar_articulation::Take::Converted)
        .last()
        .unwrap()
        .seconds;
    assert!(last_event > 0.75 + 0.01, "{last_event}");

    // a の演奏が記録されてから数える。
    wait_until("a の演奏が積まれる", || {
        !note_on_seconds(&sink, FIRST_PITCH).is_empty()
    });
    let before = note_on_seconds(&sink, FIRST_PITCH).len();
    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('z')));
    wait_until("2 周目が積まれる", || {
        note_on_seconds(&sink, FIRST_PITCH).len() >= before + 2
    });
    let on = note_on_seconds(&sink, FIRST_PITCH);
    assert!((on[before + 1] - on[before] - 0.75).abs() < 1e-6, "{on:?}");
}
