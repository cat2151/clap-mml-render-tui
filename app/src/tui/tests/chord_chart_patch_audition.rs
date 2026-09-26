//! Chord Chart の `t` / `Shift+T` の試聴で音源へ届く音。開いた直後も候補移動も、その section の
//! 進行を role のパートで鳴らしたフレーズ全体で、裏の入力欄のカーソル位置の 1 音にはならない。

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::chord_chart_preview::app_on_the_chord_chart;
use super::*;
use cmrt_mml_overlay::line_play::{
    locally_auto_voiced_bass_chord_chart_line_events, locally_auto_voiced_chord_chart_line_events,
    LinePerformance, LineStatus,
};
use cmrt_mml_overlay::{MmlOverlaySender, RecordingSink};

const NOTE_ON: u8 = 0x90;

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn attach_recording_sink(app: &mut TuiApp<'_>) -> Arc<RecordingSink> {
    let sink = Arc::new(RecordingSink::default());
    app.mml_overlay_sender = Some(MmlOverlaySender::with_recording_sink(
        Arc::clone(&sink),
        48_000.0,
    ));
    sink
}

fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done() {
        assert!(Instant::now() < deadline, "待ちきれなかった: {what}");
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// timeline へ積まれた note on の音高（受けた順）。
fn timeline_pitches(sink: &RecordingSink) -> Vec<u8> {
    sink.timeline_events()
        .iter()
        .filter(|event| event.message[0] == NOTE_ON && event.message[2] > 0)
        .map(|event| event.message[1])
        .collect()
}

/// 生 MIDI（1 音の試聴の経路）で送られた note on の音高。
fn raw_note_ons(sink: &RecordingSink) -> Vec<u8> {
    sink.midi()
        .iter()
        .filter(|(_, message)| message[0] == NOTE_ON && message[2] > 0)
        .map(|(_, message)| message[1])
        .collect()
}

fn pitches((_, performance): (LineStatus, LinePerformance)) -> Vec<u8> {
    performance
        .events
        .iter()
        .filter(|event| event.message[0] == NOTE_ON && event.message[2] > 0)
        .map(|event| event.message[1])
        .collect()
}

/// 選択中の section の進行と、その key。
fn section(app: &TuiApp<'_>) -> (String, Option<String>) {
    let degrees = app
        .chord_chart
        .selected_preview_section()
        .expect("section")
        .degrees
        .clone();
    let key =
        crate::tui::chord_chart_glue::key_token(&app.chord_chart.song.prefix).map(str::to_owned);
    (degrees, key)
}

fn app_with_patches() -> TuiApp<'static> {
    let app = app_on_the_chord_chart();
    *app.patch_load_state.lock().unwrap() = PatchLoadState::ready(make_patches(&[
        "Basses/Bass 1.fxp",
        "Basses/Bass 2.fxp",
        "Pads/Soft Pad.fxp",
        "Pads/Warm Pad.fxp",
    ]));
    app
}

/// `open` で selector を開き、開いた直後と候補移動の 2 回とも `phrase` が timeline で鳴ることを確かめる。
fn assert_both_auditions_play(
    app: &mut TuiApp<'_>,
    sink: &RecordingSink,
    open: KeyEvent,
    phrase: &[u8],
) {
    assert!(phrase.len() > 1, "フレーズのはず: {phrase:?}");
    // 1 回ずつ待つ。続けて送ると、音色の読み込み中に次の試聴が来た前の試聴は捨てられる。
    assert!(app.try_open_mml_overlay(open));
    assert!(app
        .chord_chart_patch_select
        .as_ref()
        .is_some_and(|(_, select)| select.is_select_open()));
    wait_until("開いた直後の試聴", || {
        timeline_pitches(sink).len() >= phrase.len()
    });
    app.handle_chord_chart_patch_select_key_event(plain(KeyCode::Down));
    wait_until("候補移動の試聴", || {
        timeline_pitches(sink).len() >= phrase.len() * 2
    });
    assert_eq!(timeline_pitches(sink), [phrase, phrase].concat());
    assert_eq!(sink.timelines(), 2);
    assert!(raw_note_ons(sink).is_empty(), "{:?}", raw_note_ons(sink));
}

#[test]
fn t_auditions_the_section_as_the_whole_chord_progression() {
    let mut app = app_with_patches();
    let sink = attach_recording_sink(&mut app);
    let (degrees, key) = section(&app);
    let phrase = pitches(locally_auto_voiced_chord_chart_line_events(
        &degrees,
        key.as_deref(),
        None,
    ));

    assert_both_auditions_play(&mut app, &sink, plain(KeyCode::Char('t')), &phrase);
    assert_eq!(phrase.len(), 12, "3 音 × 4 和音");
}

#[test]
fn shift_t_auditions_the_section_as_the_bass_part() {
    let mut app = app_with_patches();
    let sink = attach_recording_sink(&mut app);
    let (degrees, key) = section(&app);
    let phrase = pitches(locally_auto_voiced_bass_chord_chart_line_events(
        &degrees,
        key.as_deref(),
        None,
    ));

    assert_both_auditions_play(
        &mut app,
        &sink,
        KeyEvent::new(KeyCode::Char('T'), KeyModifiers::SHIFT),
        &phrase,
    );
    assert_eq!(phrase.len(), 4, "Bass は各 chord に 1 音");
    assert_eq!(
        sink.prepared().last().and_then(|patch| patch.patch()),
        Some("Basses/Bass 2.fxp")
    );
}

/// 進行が空の section では何も鳴らさず、音色の準備だけ（MML の試聴用 1 音は進行の音ではない）。
#[test]
fn an_empty_section_only_prepares_the_patch() {
    let mut app = app_with_patches();
    let sink = attach_recording_sink(&mut app);
    let id = app.chord_chart.selected_section().expect("section").id;
    app.chord_chart
        .song
        .section_mut(id)
        .expect("section")
        .degrees
        .clear();

    assert!(app.try_open_mml_overlay(plain(KeyCode::Char('t'))));
    wait_until("開いた直後の音色準備", || {
        !sink.prepared().is_empty()
    });
    app.handle_chord_chart_patch_select_key_event(plain(KeyCode::Down));
    wait_until("候補移動の音色準備", || {
        sink.prepared().last().and_then(|patch| patch.patch()) == Some("Pads/Warm Pad.fxp")
    });

    assert_eq!(sink.timelines(), 0);
    assert!(raw_note_ons(&sink).is_empty(), "{:?}", raw_note_ons(&sink));
}
