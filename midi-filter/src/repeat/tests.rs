use super::*;

fn event(seconds: f64, message: [u8; 3]) -> TimedMidiEvent {
    TimedMidiEvent { seconds, message }
}

#[test]
fn adds_the_same_offset_to_every_event() {
    let cycle = vec![
        event(0.0, [0x90, 60, 100]),
        event(0.5, [0x80, 60, 0]),
        event(1.5, [0x90, 64, 90]),
    ];
    let shifted = shift(&cycle, 2.0);

    assert_eq!(
        shifted.iter().map(|e| e.seconds).collect::<Vec<_>>(),
        vec![2.0, 2.5, 3.5]
    );
    assert_eq!(
        shifted.iter().map(|e| e.message).collect::<Vec<_>>(),
        cycle.iter().map(|e| e.message).collect::<Vec<_>>()
    );
}

#[test]
fn cycle_k_lands_on_k_times_the_loop_length() {
    let cycle = vec![event(0.25, [0x90, 60, 100])];
    let loop_seconds = 1.5;

    for k in 0..4 {
        let shifted = shift(&cycle, f64::from(k) * loop_seconds);
        assert_eq!(shifted[0].seconds, 0.25 + f64::from(k) * 1.5);
    }
}

#[test]
fn empty_input_stays_empty() {
    assert!(shift(&[], 3.0).is_empty());
}
