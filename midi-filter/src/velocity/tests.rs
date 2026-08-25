use super::*;

fn event(seconds: f64, message: [u8; 3]) -> TimedMidiEvent {
    TimedMidiEvent { seconds, message }
}

fn lfo() -> TriangleLfo {
    TriangleLfo::new(4.0, 0, 127)
}

#[test]
fn note_on_velocity_follows_the_lfo() {
    let mut events = vec![
        event(1.0, [0x90, 60, 100]),
        event(2.0, [0x90, 62, 100]),
        event(3.0, [0x90, 64, 100]),
    ];
    override_note_velocity(&mut events, &lfo());

    // MML が書いた 100 ではなく、その時刻の LFO 値になる。
    assert_eq!(events[0].message[2], lfo().value_at(1.0));
    assert_eq!(events[1].message[2], 127);
    assert_eq!(events[2].message[2], lfo().value_at(3.0));
    assert_ne!(events[0].message[2], 100);
}

#[test]
fn clamps_to_1_through_127() {
    let mut events: Vec<TimedMidiEvent> = (0..400)
        .map(|i| event(f64::from(i) * 0.01, [0x90, 60, 100]))
        .collect();
    override_note_velocity(&mut events, &lfo());

    assert!(events.iter().all(|e| (1..=127).contains(&e.message[2])));
    // LFO が 0 を指す時刻でも 0 にはしない（0 は note off になってしまう）。
    assert_eq!(events[0].message[2], 1);
}

#[test]
fn leaves_note_off_alone() {
    let mut events = vec![
        event(1.0, [0x80, 60, 64]),
        event(1.0, [0x90, 62, 0]),
        event(1.0, [0xB0, 1, 40]),
    ];
    override_note_velocity(&mut events, &lfo());

    assert_eq!(events[0].message, [0x80, 60, 64]);
    assert_eq!(events[1].message, [0x90, 62, 0]);
    assert_eq!(events[2].message, [0xB0, 1, 40]);
}

#[test]
fn touches_every_channel() {
    let mut events = vec![event(2.0, [0x93, 60, 100])];
    override_note_velocity(&mut events, &lfo());

    assert_eq!(events[0].message, [0x93, 60, 127]);
}
