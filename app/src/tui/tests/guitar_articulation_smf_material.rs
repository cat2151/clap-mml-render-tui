//! Guitar Articulation 画面の SMF 素材で、app が `O` で入れたパスの file を読み、画面の素材にして
//! Articulated を `play_line` へ渡すか。単音化（`M`）も同じ経路で鳴らす。
//!
//! SMF は一時ディレクトリへ `midly` で書き、実 server の代わりに記録 sender を使う。

use std::path::PathBuf;

use midly::{Format, Header, MetaMessage, MidiMessage, Smf, Timing, TrackEvent, TrackEventKind};

use super::guitar_articulation::{app_with_mml, messages, plain, sent_messages, wait_until};
use super::*;
use crate::tui::guitar_articulation::Take;

const QUARTER: u32 = 480;

fn midi(delta: u32, channel: u8, message: MidiMessage) -> TrackEvent<'static> {
    TrackEvent {
        delta: delta.into(),
        kind: TrackEventKind::Midi {
            channel: channel.into(),
            message,
        },
    }
}

fn note(delta: u32, channel: u8, key: u8, vel: u8) -> TrackEvent<'static> {
    midi(
        delta,
        channel,
        MidiMessage::NoteOn {
            key: key.into(),
            vel: vel.into(),
        },
    )
}

/// ch1 の C4 と ch2 の E4 の和音（4 分）と、CC。単音化すると E4 だけ残る。
fn poly_smf() -> Vec<u8> {
    let track = vec![
        midi(
            0,
            0,
            MidiMessage::Controller {
                controller: 7.into(),
                value: 100.into(),
            },
        ),
        note(0, 0, 60, 100),
        note(0, 1, 64, 100),
        note(QUARTER, 0, 60, 0),
        note(0, 1, 64, 0),
        TrackEvent {
            delta: 0.into(),
            kind: TrackEventKind::Meta(MetaMessage::EndOfTrack),
        },
    ];
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

fn shift(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::SHIFT)
}

/// `O` を開き、`text` を打って Enter。
fn load_by_keys(app: &mut TuiApp<'_>, text: &str) {
    app.handle_guitar_articulation_key_event(shift('O'));
    for ch in text.chars() {
        app.handle_guitar_articulation_key_event(plain(KeyCode::Char(ch)));
    }
    app.handle_guitar_articulation_key_event(plain(KeyCode::Enter));
}

fn temp_smf(tag: &str) -> (PathBuf, PathBuf) {
    let dir = crate::test_utils::unique_test_dir(tag);
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("poly song.mid");
    std::fs::write(&path, poly_smf()).unwrap();
    (dir, path)
}

#[test]
fn a_quoted_path_is_read_as_the_material_and_its_articulated_take_is_sent() {
    let (mut app, sink) = app_with_mml();
    let (dir, path) = temp_smf("ga_smf_material_load");

    load_by_keys(&mut app, &format!(" \"{}\" ", path.display()));

    std::fs::remove_dir_all(&dir).ok();
    let screen = &app.guitar_articulation;
    assert_eq!(screen.error, None);
    assert!(!screen.smf_input_open());
    assert_eq!(screen.smf_material_name(), Some("poly song.mid"));
    let notes = cmrt_chord::timed_smf_notes(&poly_smf()).unwrap().events;
    assert_eq!(screen.events(Take::Plain), notes.as_slice());
    assert_eq!(
        messages(&notes),
        vec![
            [0x90, 60, 100],
            [0x90, 64, 100],
            [0x80, 60, 0],
            [0x80, 64, 0]
        ]
    );
    let converted = screen.events(Take::Converted).to_vec();
    wait_until("Articulated", || {
        sink.timeline_events().len() >= converted.len()
    });
    assert_eq!(sent_messages(&sink, 0), messages(&converted));
    assert_eq!(sink.timelines(), 1);
}

#[test]
fn shift_m_sends_the_top_notes_and_b_then_sends_them_raw() {
    let (mut app, sink) = app_with_mml();
    let (dir, path) = temp_smf("ga_smf_material_top");
    load_by_keys(&mut app, &path.display().to_string());
    std::fs::remove_dir_all(&dir).ok();
    let first = app.guitar_articulation.events(Take::Converted).len();
    wait_until("読み込み", || sink.timeline_events().len() >= first);

    app.handle_guitar_articulation_key_event(shift('M'));

    let top = app.guitar_articulation.events(Take::Plain).to_vec();
    assert_eq!(messages(&top), vec![[0x90, 64, 100], [0x80, 64, 0]]);
    let converted = app.guitar_articulation.events(Take::Converted).to_vec();
    wait_until("単音化", || {
        sink.timeline_events().len() >= first + converted.len()
    });
    assert_eq!(sent_messages(&sink, first), messages(&converted));

    app.handle_guitar_articulation_key_event(plain(KeyCode::Char('b')));
    wait_until("raw", || {
        sink.timeline_events().len() >= first + converted.len() + top.len()
    });
    assert_eq!(
        sent_messages(&sink, first + converted.len()),
        messages(&top)
    );
    assert_eq!(sink.timelines(), 3);
}

#[test]
fn a_missing_file_keeps_the_input_open_with_the_reason_and_sends_nothing() {
    let (mut app, sink) = app_with_mml();
    let dir = crate::test_utils::unique_test_dir("ga_smf_material_missing");

    load_by_keys(&mut app, &dir.join("none.mid").display().to_string());

    let screen = &app.guitar_articulation;
    assert!(screen.smf_input_open());
    assert_eq!(screen.smf_material_name(), None);
    let error = screen.error.clone().unwrap_or_default();
    assert!(error.starts_with("none.mid: "), "{error}");
    assert_eq!(sink.timelines(), 0);
}
