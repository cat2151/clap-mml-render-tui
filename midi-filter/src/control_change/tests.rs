use super::*;

fn event(seconds: f64, message: [u8; 3]) -> TimedMidiEvent {
    TimedMidiEvent { seconds, message }
}

fn lfo() -> TriangleLfo {
    TriangleLfo::new(4.0, 0, 127)
}

#[test]
fn inserts_one_cc_per_change_point() {
    let mut events = Vec::new();
    insert_control_change(&mut events, MODULATION_CC, &lfo(), Span::new(0.0, 4.0));

    assert_eq!(events.len(), 254);
    assert!(events
        .iter()
        .all(|e| e.message[0] == 0xB0 && e.message[1] == MODULATION_CC));
    assert_eq!(events[0].message[2], 0);
    assert_eq!(events[127].message[2], 127);
}

#[test]
fn cc_comes_before_a_note_on_at_the_same_time() {
    let mut events = vec![
        event(0.0, [0x90, 60, 100]),
        event(2.0, [0x80, 60, 0]),
        event(2.0, [0x90, 64, 100]),
    ];
    insert_control_change(&mut events, MODULATION_CC, &lfo(), Span::new(0.0, 4.0));

    // span の頭に置いた CC が、同時刻の note on より前に来る。
    assert_eq!(events[0].message, [0xB0, MODULATION_CC, 0]);
    assert_eq!(events[1].message, [0x90, 60, 100]);

    // 折り返し（2.0 秒）では note off → CC → note on の順。
    let fold: Vec<[u8; 3]> = events
        .iter()
        .filter(|e| e.seconds == 2.0)
        .map(|e| e.message)
        .collect();
    assert_eq!(
        fold,
        vec![[0x80, 60, 0], [0xB0, MODULATION_CC, 127], [0x90, 64, 100]]
    );
}

#[test]
fn keeps_the_existing_events_in_time_order() {
    let mut events = vec![
        event(0.5, [0x90, 60, 100]),
        event(1.5, [0x80, 60, 0]),
        event(3.5, [0x90, 67, 90]),
    ];
    insert_control_change(&mut events, MODULATION_CC, &lfo(), Span::new(0.0, 4.0));

    let notes: Vec<f64> = events
        .iter()
        .filter(|e| e.message[0] != 0xB0)
        .map(|e| e.seconds)
        .collect();
    assert_eq!(notes, vec![0.5, 1.5, 3.5]);
    for pair in events.windows(2) {
        assert!(pair[1].seconds >= pair[0].seconds);
    }
}

#[test]
fn data_bytes_stay_inside_7_bits() {
    let mut events = Vec::new();
    insert_control_change(&mut events, 0xFF, &lfo(), Span::new(0.0, 4.0));

    assert!(events
        .iter()
        .all(|e| e.message[1] < 0x80 && e.message[2] < 0x80));
    assert_eq!(events[0].message[1], 0x7F);
}
