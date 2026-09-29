use super::*;

fn event(seconds: f64, message: [u8; 3]) -> TimedMidiEvent {
    TimedMidiEvent { seconds, message }
}

#[test]
fn pairs_on_and_off_and_numbers_columns_by_on_time() {
    let events = vec![
        event(0.0, [0x90, 40, 100]),
        event(0.5, [0x80, 40, 0]),
        event(0.5, [0x90, 42, 90]),
        event(0.5, [0x90, 45, 90]),
        event(1.0, [0x80, 42, 0]),
        event(1.0, [0x90, 45, 0]),
    ];
    let notes = notes_from_events(&events);

    let summary: Vec<(f64, f64, u8, u8, usize)> = notes
        .iter()
        .map(|n| (n.on_seconds, n.off_seconds, n.pitch, n.velocity, n.column))
        .collect();
    assert_eq!(
        summary,
        vec![
            (0.0, 0.5, 40, 100, 0),
            (0.5, 1.0, 42, 90, 1),
            (0.5, 1.0, 45, 90, 1),
        ]
    );
}

#[test]
fn repeated_pitch_closes_the_oldest_open_note() {
    let events = vec![
        event(0.0, [0x90, 40, 100]),
        event(0.5, [0x80, 40, 0]),
        event(0.5, [0x90, 40, 100]),
        event(1.0, [0x80, 40, 0]),
    ];
    let notes = notes_from_events(&events);

    assert_eq!(notes.len(), 2);
    assert_eq!((notes[0].off_seconds, notes[1].off_seconds), (0.5, 1.0));
    assert_eq!((notes[0].column, notes[1].column), (0, 1));
}

#[test]
fn unclosed_note_ends_at_the_last_event() {
    let events = vec![event(0.0, [0x91, 40, 100]), event(2.0, [0xB1, 1, 0])];
    let notes = notes_from_events(&events);

    assert_eq!(notes[0].off_seconds, 2.0);
    assert_eq!(notes[0].channel, 1);
}
