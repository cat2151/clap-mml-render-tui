use super::*;

fn at(seconds: f64, message: [u8; 3]) -> TimedMidiEvent {
    TimedMidiEvent { seconds, message }
}

fn midi(events: Vec<TimedMidiEvent>) -> SampleMidi {
    SampleMidi::new("test.mid".to_string(), events)
}

/// 3 つの音。1 つ目の区間に CC20 = 10 → 50 と KS 20、2 つ目の区間に CC20 = 70 と pitch bend。
fn three_notes(with_bend: bool) -> Vec<TimedMidiEvent> {
    let mut events = vec![
        at(0.0, [0xB0, 20, 10]),
        at(0.0, [0x90, 20, 127]),
        at(0.0, [0x90, 60, 100]),
        at(0.5, [0xB0, 20, 50]),
        at(1.0, [0x80, 60, 0]),
        at(1.0, [0x80, 20, 0]),
        at(1.0, [0x90, 62, 100]),
        at(1.5, [0xB0, 20, 70]),
    ];
    if with_bend {
        // 9000 - 8192 = +808。
        events.push(at(1.5, [0xE0, 40, 70]));
    }
    events.extend([
        at(2.0, [0x80, 62, 0]),
        at(2.0, [0x90, 64, 100]),
        at(3.0, [0x80, 64, 0]),
    ]);
    events
}

#[test]
fn a_keyswitch_on_the_same_second_is_not_a_note_of_its_own() {
    let sample = midi(vec![
        at(0.0, [0x90, 17, 127]),
        at(0.0, [0x90, 60, 100]),
        at(0.5, [0x80, 60, 0]),
        at(0.5, [0x80, 17, 0]),
    ]);

    assert_eq!(sample.group_count(), 1);
    assert_eq!(sample.group_event_index(0), Some(1));
    assert_eq!(sample.group_on_seconds(0), Some(0.0));
}

#[test]
fn a_two_note_chord_is_one_group() {
    let sample = midi(vec![
        at(0.0, [0x90, 40, 100]),
        at(0.0, [0x90, 47, 100]),
        at(0.5, [0x80, 40, 0]),
        at(0.5, [0x80, 47, 0]),
        at(0.5, [0x90, 42, 100]),
        at(1.0, [0x80, 42, 0]),
    ]);

    assert_eq!(sample.group_count(), 2);
    assert_eq!(sample.group_on_seconds(1), Some(0.5));
}

#[test]
fn one_note_resends_the_state_before_it_and_keeps_its_own_span_only() {
    let sample = midi(three_notes(true));

    assert_eq!(
        sample.note_events(1),
        vec![
            // 区間の頭の KS 20 の note off（前の音の終わり）。KS はラッチなので害は無い。
            at(0.0, [0x80, 20, 0]),
            at(0.0, [0xB0, 20, 50]),
            // 前に pitch bend は無いが、MID のどこかにあるので中央へ戻す。
            at(0.0, [0xE0, 0x00, 0x40]),
            at(0.0, [0x90, 20, 127]),
            at(0.0, [0x90, 62, 100]),
            at(0.5, [0xB0, 20, 70]),
            at(0.5, [0xE0, 40, 70]),
            // 送り直した KS はこの音の終わりで離す。
            at(1.0, [0x80, 20, 0]),
            at(1.0, [0x80, 62, 0]),
            // 2.0 秒の 64 の note on（次の音）は入らない。
        ]
    );
}

#[test]
fn a_later_pitch_bend_before_the_note_is_resent_instead_of_the_center() {
    let mut events = three_notes(true);
    events.push(at(3.5, [0x90, 65, 100]));
    events.push(at(4.0, [0x80, 65, 0]));
    let sample = midi(events);

    let resent: Vec<TimedMidiEvent> = sample
        .note_events(3)
        .into_iter()
        .filter(|event| event.seconds == 0.0 && event.message[0] == 0xE0)
        .collect();
    assert_eq!(resent, vec![at(0.0, [0xE0, 40, 70])]);
}

#[test]
fn the_first_note_has_only_its_span_when_nothing_comes_before() {
    let sample = midi(three_notes(false));

    assert_eq!(
        sample.note_events(0),
        vec![
            at(0.0, [0xB0, 20, 10]),
            at(0.0, [0x90, 20, 127]),
            at(0.0, [0x90, 60, 100]),
            at(0.5, [0xB0, 20, 50]),
            at(1.0, [0x80, 60, 0]),
            at(1.0, [0x80, 20, 0]),
        ]
    );
}

#[test]
fn a_group_out_of_range_has_no_events() {
    let sample = midi(three_notes(false));

    assert_eq!(sample.group_count(), 3);
    assert!(sample.note_events(3).is_empty());
    assert_eq!(sample.group_on_seconds(3), None);
}

#[test]
fn keyswitch_names_follow_the_ks_map() {
    assert_eq!(keyswitch_name(0), Some("Hello!"));
    assert_eq!(keyswitch_name(14), Some("Mute_Fret_D"));
    assert_eq!(keyswitch_name(29), Some("Pseudo_Legato"));
    assert_eq!(keyswitch_name(30), None);
    assert_eq!(keyswitch_name(90), None);
    assert_eq!(keyswitch_name(91), Some("Bending_HT"));
    assert_eq!(keyswitch_name(102), Some("Trill_Maj3"));
    assert_eq!(keyswitch_name(103), None);
    assert!(is_keyswitch_pitch(22));
    assert!(!is_keyswitch_pitch(35));
}
