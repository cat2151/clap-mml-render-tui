use super::*;
use Articulation::{HammerOn, PullOff, SusDown};

fn note(on: f64, pitch: u8, column: usize) -> Note {
    Note {
        on_seconds: on,
        off_seconds: on + 0.5,
        channel: 0,
        pitch,
        velocity: 100,
        column,
    }
}

fn three_notes() -> Vec<Note> {
    vec![note(0.0, 40, 0), note(0.5, 42, 1), note(1.0, 40, 2)]
}

fn ks_on(seconds: f64, key: u8) -> TimedMidiEvent {
    TimedMidiEvent {
        seconds,
        message: [0x90, key, KEYSWITCH_VELOCITY],
    }
}

fn ks_off(seconds: f64, key: u8) -> TimedMidiEvent {
    TimedMidiEvent {
        seconds,
        message: [0x80, key, 0],
    }
}

#[test]
fn all_default_puts_a_single_sus_down_at_the_head() {
    let events = keyswitch_events(&three_notes(), &[SusDown, SusDown, SusDown]);
    assert_eq!(events, vec![ks_on(0.0, 17), ks_off(0.5, 17)]);
}

#[test]
fn keyswitch_only_where_the_articulation_changes() {
    let events = keyswitch_events(&three_notes(), &[SusDown, HammerOn, HammerOn]);
    assert_eq!(
        events,
        vec![
            ks_on(0.0, 17),
            ks_off(0.5, 17),
            ks_on(0.5, 26),
            ks_off(1.0, 26),
            // 最後が Hammer-On のままなので、末尾で既定へ戻す。
            ks_on(1.5, 17),
            ks_off(1.5 + KEYSWITCH_RESET_SECONDS, 17),
        ]
    );
}

#[test]
fn ending_on_the_default_does_not_add_a_reset() {
    let events = keyswitch_events(&three_notes(), &[SusDown, PullOff, SusDown]);
    assert_eq!(
        events,
        vec![
            ks_on(0.0, 17),
            ks_off(0.5, 17),
            ks_on(0.5, 25),
            ks_off(1.0, 25),
            ks_on(1.0, 17),
            ks_off(1.5, 17),
        ]
    );
}

#[test]
fn chord_column_gets_one_keyswitch() {
    let notes = vec![note(0.0, 40, 0), note(0.0, 45, 0)];
    let events = keyswitch_events(&notes, &[SusDown, SusDown]);
    assert_eq!(events, vec![ks_on(0.0, 17), ks_off(0.5, 17)]);
}

#[test]
fn keyswitch_uses_the_note_channel() {
    let mut n = note(0.0, 40, 0);
    n.channel = 3;
    let events = keyswitch_events(&[n], &[SusDown]);
    assert_eq!(events[0].message, [0x93, 17, KEYSWITCH_VELOCITY]);
    assert_eq!(events[1].message, [0x83, 17, 0]);
}

#[test]
fn no_notes_no_keyswitch() {
    assert!(keyswitch_events(&[], &[]).is_empty());
}
