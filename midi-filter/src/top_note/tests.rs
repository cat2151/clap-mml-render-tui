use super::*;

const C4: u8 = 60;
const D4: u8 = 62;
const E4: u8 = 64;
const G4: u8 = 67;

fn on(seconds: f64, pitch: u8) -> TimedMidiEvent {
    TimedMidiEvent {
        seconds,
        message: [0x90, pitch, 100],
    }
}

fn off(seconds: f64, pitch: u8) -> TimedMidiEvent {
    TimedMidiEvent {
        seconds,
        message: [0x80, pitch, 0],
    }
}

/// `(on, off, pitch)` の組を、note on / note off のイベント列にする。
fn notes(spans: &[(f64, f64, u8)]) -> Vec<TimedMidiEvent> {
    let mut events: Vec<TimedMidiEvent> = spans
        .iter()
        .flat_map(|&(start, end, pitch)| [on(start, pitch), off(end, pitch)])
        .collect();
    sort_for_playback(&mut events);
    events
}

#[test]
fn a_chord_keeps_only_its_top_note() {
    let result = keep_top_notes(&notes(&[(0.0, 1.0, C4), (0.0, 1.0, E4), (0.0, 1.0, G4)]));

    assert_eq!(result, notes(&[(0.0, 1.0, G4)]));
}

#[test]
fn a_lower_note_is_cut_where_a_higher_note_starts_and_does_not_sound_again() {
    let result = keep_top_notes(&notes(&[(0.0, 2.0, C4), (0.5, 1.0, E4)]));

    assert_eq!(result, notes(&[(0.0, 0.5, C4), (0.5, 1.0, E4)]));
}

#[test]
fn a_note_that_starts_under_a_sounding_higher_note_is_removed() {
    let result = keep_top_notes(&notes(&[(0.0, 1.0, E4), (0.5, 1.5, C4)]));

    assert_eq!(result, notes(&[(0.0, 1.0, E4)]));
}

#[test]
fn a_legato_overlap_in_a_melody_is_trimmed_and_both_notes_stay() {
    let result = keep_top_notes(&notes(&[(0.0, 1.02, C4), (1.0, 2.0, D4)]));

    assert_eq!(result, notes(&[(0.0, 1.0, C4), (1.0, 2.0, D4)]));
}

#[test]
fn an_empty_input_stays_empty() {
    assert!(keep_top_notes(&[]).is_empty());
}

#[test]
fn a_single_melody_line_passes_through_unchanged() {
    let melody = notes(&[(0.0, 0.5, C4), (0.5, 1.0, E4), (1.0, 1.5, D4)]);

    assert_eq!(keep_top_notes(&melody), melody);
}

#[test]
fn overlapping_notes_of_the_same_pitch_pair_oldest_first_and_both_stay() {
    // on 0 / on 1 / off 2 / off 3。最も古い note on から順に off を当てるので
    // 0〜2 と 1〜3 の 2 音。同じ音高は「より高い音」ではないので互いに切らない。
    let events = notes(&[(0.0, 2.0, C4), (1.0, 3.0, C4)]);

    assert_eq!(keep_top_notes(&events), events);
}

#[test]
fn a_higher_note_counts_only_while_it_still_sounds_after_trimming() {
    // E4 は 1 秒で G4 に切られて鳴り止む。C4 が 2 秒で鳴り始めたとき、
    // 元の E4 は 3 秒まであるが、単音化後は鳴っていないので C4 は残る。
    let result = keep_top_notes(&notes(&[(0.0, 3.0, E4), (1.0, 1.5, G4), (2.0, 3.0, C4)]));

    assert_eq!(
        result,
        notes(&[(0.0, 1.0, E4), (1.0, 1.5, G4), (2.0, 3.0, C4)])
    );
}

#[test]
fn a_note_without_a_note_off_is_closed_where_it_is_trimmed() {
    let result = keep_top_notes(&[on(0.0, C4), on(0.5, E4), off(1.0, E4)]);

    assert_eq!(result, notes(&[(0.0, 0.5, C4), (0.5, 1.0, E4)]));
}

#[test]
fn a_zero_velocity_note_on_ends_a_note_and_keeps_its_own_bytes() {
    let release = TimedMidiEvent {
        seconds: 1.0,
        message: [0x90, C4, 0],
    };
    let events = vec![on(0.0, C4), release];

    assert_eq!(keep_top_notes(&events), events);
}

#[test]
fn events_other_than_notes_are_kept() {
    let control = TimedMidiEvent {
        seconds: 0.25,
        message: [0xB0, 1, 64],
    };
    let mut events = notes(&[(0.0, 1.0, C4), (0.0, 1.0, E4)]);
    events.push(control);

    let mut expected = notes(&[(0.0, 1.0, E4)]);
    expected.push(control);
    sort_for_playback(&mut expected);
    assert_eq!(keep_top_notes(&events), expected);
}
