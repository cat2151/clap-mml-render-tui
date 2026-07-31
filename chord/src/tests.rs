use super::*;
use midly::{Format, Header, Timing, TrackEvent, TrackEventKind};

#[test]
fn sequential_mml_notes_become_single_note_chords_in_order() {
    assert_eq!(
        note_progression("cec").unwrap(),
        vec![vec![60], vec![64], vec![60]]
    );
}

#[test]
fn chord_notation_is_preprocessed_before_mml_parsing() {
    assert_eq!(note_progression("C").unwrap(), vec![vec![60, 64, 67]]);
}

#[test]
fn regular_mml_passes_through_when_chord_conversion_fails() {
    assert_eq!(
        note_progression("ceg").unwrap(),
        vec![vec![60], vec![64], vec![67]]
    );
}

#[test]
fn empty_and_rest_only_mml_are_rejected() {
    assert_eq!(note_progression(" ").unwrap_err(), "MMLを入力してください");
    assert_eq!(
        note_progression("r").unwrap_err(),
        "MMLに発音ノートがありません"
    );
}

#[test]
fn smf_tracks_are_merged_into_chords_and_zero_velocity_is_ignored() {
    let header = Header::new(Format::Parallel, Timing::Metrical(480.into()));
    let tracks = vec![
        vec![
            note_on(10, 67, 100),
            note_on(0, 69, 0),
            note_on(20, 67, 100),
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(midly::MetaMessage::EndOfTrack),
            },
        ],
        vec![
            note_on(10, 60, 100),
            note_on(0, 60, 100),
            note_on(20, 64, 100),
            TrackEvent {
                delta: 0.into(),
                kind: TrackEventKind::Meta(midly::MetaMessage::EndOfTrack),
            },
        ],
    ];
    let mut bytes = Vec::new();
    Smf { header, tracks }.write_std(&mut bytes).unwrap();

    assert_eq!(
        note_progression_from_smf(&bytes).unwrap(),
        vec![vec![67, 60], vec![67, 64]]
    );
}

fn note_on(delta: u32, key: u8, velocity: u8) -> TrackEvent<'static> {
    TrackEvent {
        delta: delta.into(),
        kind: TrackEventKind::Midi {
            channel: 0.into(),
            message: MidiMessage::NoteOn {
                key: key.into(),
                vel: velocity.into(),
            },
        },
    }
}
