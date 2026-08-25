use super::*;

fn event(seconds: f64, message: [u8; 3]) -> TimedMidiEvent {
    TimedMidiEvent { seconds, message }
}

#[test]
fn same_time_orders_note_off_then_cc_then_note_on() {
    let mut events = vec![
        event(1.0, [0x90, 62, 100]),
        event(1.0, [0xB0, 1, 40]),
        event(1.0, [0x80, 60, 0]),
        event(0.5, [0x90, 60, 100]),
    ];
    sort_for_playback(&mut events);

    let order: Vec<[u8; 3]> = events.iter().map(|e| e.message).collect();
    assert_eq!(
        order,
        vec![
            [0x90, 60, 100],
            [0x80, 60, 0],
            [0xB0, 1, 40],
            [0x90, 62, 100],
        ]
    );
}

#[test]
fn velocity_zero_note_on_is_treated_as_note_off() {
    let mut events = vec![event(1.0, [0x90, 60, 100]), event(1.0, [0x90, 60, 0])];
    sort_for_playback(&mut events);

    // 同じ音高の連打で、消音が新しい発音より後ろに回らないこと。
    assert_eq!(events[0].message, [0x90, 60, 0]);
    assert_eq!(events[1].message, [0x90, 60, 100]);
}

#[test]
fn sort_is_stable_within_the_same_kind() {
    let mut events = vec![
        event(1.0, [0xB0, 1, 10]),
        event(1.0, [0xB0, 1, 11]),
        event(1.0, [0xB0, 1, 12]),
        event(0.0, [0xB0, 1, 9]),
    ];
    sort_for_playback(&mut events);

    let values: Vec<u8> = events.iter().map(|e| e.message[2]).collect();
    assert_eq!(values, vec![9, 10, 11, 12]);
}

#[test]
fn span_duration_never_goes_negative() {
    assert_eq!(Span::new(1.0, 3.5).duration_seconds(), 2.5);
    assert_eq!(Span::new(3.5, 1.0).duration_seconds(), 0.0);
}
