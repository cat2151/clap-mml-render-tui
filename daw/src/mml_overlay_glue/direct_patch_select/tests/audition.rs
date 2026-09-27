//! `t` の試聴で音源へ届く音。開いた直後も候補移動も、その meas のフレーズ全体が鳴り、
//! 裏の入力欄のカーソル位置の 1 音にはならない。

use cmrt_mml_overlay::line_play::line_events;
use cmrt_mml_overlay::RecordingSink;
use crossterm::event::KeyCode;

use super::*;

const NOTE_ON: u8 = 0x90;

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

/// その meas を DAW が鳴らす MML の note on の音高。
fn measure_pitches(app: &DawApp, track: usize, measure: usize) -> Vec<u8> {
    let line = crate::mml::cell_preview_line(&app.editor.data, track, measure);
    line_events(&line)
        .1
        .events
        .iter()
        .filter(|event| event.message[0] == NOTE_ON && event.message[2] > 0)
        .map(|event| event.message[1])
        .collect()
}

/// `t` → 候補移動で、開いた直後と移動後の 2 回ともその meas のフレーズが timeline で鳴る。
fn assert_both_auditions_play_the_measure(app: &mut DawApp, sink: &RecordingSink, step: KeyCode) {
    let phrase = measure_pitches(app, 2, 1);
    assert!(phrase.len() > 1, "フレーズのはず: {phrase:?}");
    // 1 回ずつ待つ。続けて送ると、音色の読み込み中に次の試聴が来た前の試聴は捨てられる。
    app.handle_normal_key_event(plain('t'));
    wait_until("開いた直後の試聴", || {
        timeline_pitches(sink).len() >= phrase.len()
    });
    app.handle_direct_patch_select_key_event(key(step));
    wait_until("候補移動の試聴", || {
        timeline_pitches(sink).len() >= phrase.len() * 2
    });
    let heard = timeline_pitches(sink);
    assert_eq!(heard, [phrase.clone(), phrase].concat());
    assert_eq!(sink.timelines(), 2);
    assert!(raw_note_ons(sink).is_empty(), "{:?}", raw_note_ons(sink));
}

#[test]
fn a_written_kick_measure_is_auditioned_as_the_whole_phrase() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_drum_catalog(r#"{"Surge XT patch": "Drums/Kick A.fxp"}"#);
    app.editor.data[2][1] = "l8o2cccc".to_string();
    let sink = attach_recording_sink(&mut app);

    assert_both_auditions_play_the_measure(&mut app, &sink, KeyCode::Down);
    assert_eq!(
        sink.prepared().last().and_then(|patch| patch.patch()),
        Some("Drums/Kick B.fxp")
    );
}

#[test]
fn a_measure_generated_from_the_chord_row_is_auditioned_as_the_whole_phrase() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_generated_empty_cell();
    let sink = attach_recording_sink(&mut app);

    // 一覧は Keys, Pad の順でカーソルは Pad にあるので、上へ動かす。
    assert_both_auditions_play_the_measure(&mut app, &sink, KeyCode::Up);
    assert_eq!(
        sink.prepared().last().and_then(|patch| patch.patch()),
        Some("Pads/Snapshot Keys.fxp")
    );
}

#[test]
fn the_init_column_auditions_the_single_preview_note() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_pad_track();
    app.editor.cursor_measure = 0;
    let sink = attach_recording_sink(&mut app);

    app.handle_normal_key_event(plain('t'));
    wait_until("開いた直後の 1 音", || {
        !raw_note_ons(&sink).is_empty()
    });
    app.handle_direct_patch_select_key_event(key(KeyCode::Up));
    wait_until("候補移動の 1 音", || raw_note_ons(&sink).len() >= 2);
    assert_eq!(raw_note_ons(&sink), [60, 60]);
    assert_eq!(sink.timelines(), 0);
}
