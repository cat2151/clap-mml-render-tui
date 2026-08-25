use super::*;
use midly::{Format, Header, TrackEvent};

const QUARTER: u32 = 480;

#[test]
fn a_chord_name_is_recognised_as_a_chord() {
    let performance = timed_performance("C").unwrap();

    assert!(performance.from_chord);
    let pitches = note_on_pitches(&performance);
    assert_eq!(pitches, vec![60, 64, 67]);
}

#[test]
fn plain_mml_passes_through_as_mml() {
    let performance = timed_performance("ceg").unwrap();

    assert!(!performance.from_chord);
    assert_eq!(note_on_pitches(&performance), vec![60, 64, 67]);
}

#[test]
fn notes_are_ordered_in_time_and_every_note_on_is_released() {
    let performance = timed_performance("cde").unwrap();

    // 小文字の音名だけの MML は chord 表記と紛れやすい。単音3つのまま通ること。
    assert!(!performance.from_chord);
    let mut previous = -1.0;
    for event in &performance.events {
        assert!(event.seconds >= previous, "{:?}", performance.events);
        previous = event.seconds;
    }
    let note_ons = performance
        .events
        .iter()
        .filter(|event| event.message[0] == NOTE_ON)
        .count();
    let note_offs = performance
        .events
        .iter()
        .filter(|event| event.message[0] == NOTE_OFF)
        .count();
    assert_eq!(note_ons, 3);
    assert_eq!(note_offs, 3);
    assert!(performance.duration_seconds > 0.0);
}

#[test]
fn empty_input_is_rejected() {
    assert_eq!(
        timed_performance("  ").unwrap_err(),
        "MMLを入力してください"
    );
}

#[test]
fn a_rest_only_phrase_has_no_notes() {
    assert_eq!(
        timed_performance("r").unwrap_err(),
        "MMLに発音ノートがありません"
    );
}

/// 既定テンポ（120 BPM）では 4 分音符 = 0.5 秒。
#[test]
fn ticks_become_seconds_at_the_default_tempo() {
    let (events, duration) = timed_events_from_smf(&smf_bytes(vec![
        note_on(0, 60, 100),
        note_on(QUARTER, 60, 0),
        end_of_track(0),
    ]))
    .unwrap();

    assert_eq!(events[0].seconds, 0.0);
    assert_eq!(events[1].seconds, 0.5);
    assert_eq!(duration, 0.5);
}

#[test]
fn a_tempo_change_bends_the_seconds_after_it() {
    // 120 BPM で 1 拍 → 240 BPM へ切り替えて 1 拍。後半は半分の時間になる。
    let (events, _) = timed_events_from_smf(&smf_bytes(vec![
        note_on(0, 60, 100),
        note_on(QUARTER, 60, 0),
        tempo(0, 250_000),
        note_on(0, 62, 100),
        note_on(QUARTER, 62, 0),
        end_of_track(0),
    ]))
    .unwrap();

    assert_eq!(events[1].seconds, 0.5);
    assert_eq!(events[3].seconds, 0.75);
}

/// 同じ音高を続けて鳴らすとき、前の音の note off が新しい note on より後ろに
/// 並ぶと発音が消える。同時刻では必ず note off を先に出す。
#[test]
fn a_note_off_comes_before_a_note_on_at_the_same_tick() {
    let (events, _) = timed_events_from_smf(&smf_bytes(vec![
        note_on(0, 60, 100),
        note_on(QUARTER, 60, 100),
        note_on(0, 60, 0),
        end_of_track(0),
    ]))
    .unwrap();

    assert_eq!(events[1].message, [NOTE_OFF, 60, 0]);
    assert_eq!(events[2].message, [NOTE_ON, 60, 100]);
}

#[test]
fn a_phrase_without_notes_is_rejected() {
    assert_eq!(
        timed_events_from_smf(&smf_bytes(vec![end_of_track(QUARTER)])).unwrap_err(),
        "MMLに発音ノートがありません"
    );
}

fn note_on_pitches(performance: &TimedPerformance) -> Vec<u8> {
    performance
        .events
        .iter()
        .filter(|event| event.message[0] == NOTE_ON)
        .map(|event| event.message[1])
        .collect()
}

fn smf_bytes(track: Vec<TrackEvent<'static>>) -> Vec<u8> {
    let mut bytes = Vec::new();
    Smf {
        header: Header::new(
            Format::SingleTrack,
            Timing::Metrical((QUARTER as u16).into()),
        ),
        tracks: vec![track],
    }
    .write_std(&mut bytes)
    .unwrap();
    bytes
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

fn tempo(delta: u32, micros_per_beat: u32) -> TrackEvent<'static> {
    TrackEvent {
        delta: delta.into(),
        kind: TrackEventKind::Meta(MetaMessage::Tempo(micros_per_beat.into())),
    }
}

fn end_of_track(delta: u32) -> TrackEvent<'static> {
    TrackEvent {
        delta: delta.into(),
        kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
    }
}

/// 移設の要点は「同じ型であること」。`cmrt_chord::TimedMidiEvent` が
/// `cmrt_midi_filter` の型そのものなら、詰め替えなしで filter へ渡せる。
/// 将来うっかり chord 側へ同名の型を再定義したらここが型エラーで落ちる。
#[test]
fn the_events_go_straight_into_a_midi_filter_without_conversion() {
    let performance = timed_performance("cde").unwrap();

    let shifted: Vec<cmrt_midi_filter::TimedMidiEvent> =
        cmrt_midi_filter::shift(&performance.events, 1.5);

    assert_eq!(shifted.len(), performance.events.len());
    assert_eq!(shifted[0].seconds, performance.events[0].seconds + 1.5);
    assert_eq!(shifted[0].message, performance.events[0].message);
}
