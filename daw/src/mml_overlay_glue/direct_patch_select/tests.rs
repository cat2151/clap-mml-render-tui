//! NORMAL の `t` から音色 selector だけを開く経路の検証。
//!
//! guard の握り方は `mml_overlay_glue/tests.rs` の注意書きと同じ（1 テスト = 1 guard、
//! テスト関数の先頭）。

use std::sync::Arc;
use std::time::{Duration, Instant};

use cmrt_mml_overlay::line_play::LineStatus;
use cmrt_mml_overlay::{MmlOverlaySender, RecordingSink};
use cmrt_tui_core::patch_load::PatchLoadState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::super::{DawApp, DawMode};
use crate::input::tests::build_test_app;

const PAD_INIT_CELL: &str = r#"{"Surge XT patch": "Pads/Snapshot Pad.fxp"}"#;
const BASS_INIT_CELL: &str = r#"{"Surge XT patch": "Bass/Snapshot Bass.fxp"}"#;

pub(in crate::mml_overlay_glue) fn plain(code: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(code), KeyModifiers::NONE)
}

pub(in crate::mml_overlay_glue) fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn catalog_pairs() -> Vec<(String, String)> {
    ["Bass/Snapshot Bass.fxp", "Pads/Snapshot Pad.fxp"]
        .into_iter()
        .map(|display| (display.to_string(), display.to_lowercase()))
        .collect()
}

fn app_with_pad_track() -> (DawApp, std::sync::mpsc::Receiver<crate::CacheJob>) {
    let (mut app, cache_rx) = build_test_app();
    *app.patch_load.lock().unwrap() = PatchLoadState::ready(catalog_pairs());
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    app.editor.data[2][0] = PAD_INIT_CELL.to_string();
    app.editor.data[2][1] = "cde".to_string();
    (app, cache_rx)
}

#[test]
fn t_opens_the_patch_selector_without_the_mml_input_step() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_pad_track();

    app.handle_normal_key_event(plain('t'));

    assert_eq!(app.mode, DawMode::MmlOverlay);
    assert!(app.mml_overlay.is_patch_select_open());
    assert_eq!(app.mml_overlay.patch(), Some("Pads/Snapshot Pad.fxp"));
}

#[test]
fn confirming_writes_the_init_cell_and_returns_to_normal() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_pad_track();
    app.handle_normal_key_event(plain('t'));

    app.handle_mml_overlay_key_event(plain('/'));
    for ch in "snapshot bass".chars() {
        app.handle_mml_overlay_key_event(plain(ch));
    }
    // 1 回目は絞り込み、2 回目は音色の確定。
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));
    app.handle_mml_overlay_key_event(key(KeyCode::Enter));

    assert_eq!(app.editor.data[2][0], BASS_INIT_CELL);
    assert_eq!(app.mode, DawMode::Normal);
    assert!(!app.mml_overlay.is_open());
    assert!(!app.mml_overlay_patch_select_only);
    // 下敷きの入力欄の中身をセルへ書き戻していないこと。
    assert_eq!(app.editor.data[2][1], "cde");
    assert_eq!(app.editor.cursor_measure, 1);
    assert_eq!(*app.playback.auto_play_reservation.lock().unwrap(), Some(1));
}

#[test]
fn cancelling_keeps_the_init_cell_and_returns_to_normal() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_pad_track();
    app.handle_normal_key_event(plain('t'));

    app.handle_mml_overlay_key_event(key(KeyCode::Esc));

    assert_eq!(app.editor.data[2][0], PAD_INIT_CELL);
    assert_eq!(app.mode, DawMode::Normal);
    assert!(!app.mml_overlay.is_open());
    assert_eq!(*app.playback.auto_play_reservation.lock().unwrap(), None);
}

#[test]
fn the_init_column_opens_the_selector_with_an_empty_preview_line() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_pad_track();
    app.editor.cursor_measure = 0;

    app.handle_normal_key_event(plain('t'));

    assert!(app.mml_overlay.is_patch_select_open());
    assert_eq!(app.mml_overlay.value(), "");
}

#[test]
fn the_tempo_row_does_not_open_the_selector() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_pad_track();
    app.editor.cursor_track = 0;

    app.handle_normal_key_event(plain('t'));

    assert_eq!(app.mode, DawMode::Normal);
    assert!(!app.mml_overlay.is_open());
}

#[test]
fn loading_catalog_waits_and_esc_closes_without_touching_cells() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_pad_track();
    *app.patch_load.lock().unwrap() = PatchLoadState::Loading;

    app.handle_normal_key_event(plain('t'));
    assert_eq!(app.mode, DawMode::MmlOverlay);
    assert!(app.mml_overlay.is_waiting_for_patch_catalog());

    // 待っている間の文字キーは入力欄へ通さない。
    app.handle_mml_overlay_key_event(plain('x'));
    assert_eq!(app.mml_overlay.value(), "cde");

    app.handle_mml_overlay_key_event(key(KeyCode::Esc));
    assert_eq!(app.mode, DawMode::Normal);
    assert_eq!(app.editor.data[2][1], "cde");
}

#[test]
fn an_empty_catalog_does_not_leave_the_overlay_open() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_pad_track();
    *app.patch_load.lock().unwrap() = PatchLoadState::ready(Vec::new());

    app.handle_normal_key_event(plain('t'));

    assert_eq!(app.mode, DawMode::Normal);
    assert!(!app.mml_overlay.is_open());
}

#[test]
fn an_empty_cell_generated_from_the_chord_row_previews_the_generated_notes() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_pad_track();
    app.editor.data[1][1] = "Cm7".to_string();
    app.editor.data[2][0] =
        r#"{"Surge XT patch": "Pads/Snapshot Pad.fxp", "generate from chord track": "close"}o3"#
            .to_string();
    app.editor.data[2][1].clear();

    app.handle_normal_key_event(plain('t'));

    let generated = crate::mml::chord_generation::generate_mml_from_chord_cell("", "close", "Cm7");
    assert!(!generated.is_empty());
    let expected = generated
        .split(';')
        .map(|part| format!("o3{}", part.trim()))
        .collect::<Vec<_>>()
        .join(";");
    assert!(app.mml_overlay.is_patch_select_open());
    assert_eq!(app.mml_overlay.value(), expected);
    // overlay の解釈で音符になること（無音行なら Space の試聴が鳴らない）。
    assert!(cmrt_mml_overlay::live_line(&expected).is_ok());
}

/// chord 行 `I` から `"close"` で生成される track2 の meas1（セルは空）。
pub(in crate::mml_overlay_glue) fn app_with_generated_empty_cell(
) -> (DawApp, std::sync::mpsc::Receiver<crate::CacheJob>) {
    let (mut app, cache_rx) = app_with_pad_track();
    app.editor.data[1][1] = "I".to_string();
    app.editor.data[2][0] =
        r#"{"Surge XT patch":"Pads/Snapshot Pad.fxp","generate from chord track":"close"}"#
            .to_string();
    app.editor.data[2][1].clear();
    (app, cache_rx)
}

#[test]
fn moving_in_the_selector_plays_the_generated_chord_not_a_single_note() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_generated_empty_cell();
    app.handle_normal_key_event(plain('t'));

    // 生成行は `/*|*/` で終わるので、行末のカーソルはどの発音単位にも触れていない。
    // 一覧は Bass, Pad の順でカーソルは Pad にあるので、上へ動かす。
    app.handle_mml_overlay_key_event(key(KeyCode::Up));

    // 行全体を鳴らした（C の 3 和音）。試聴用の単音へ落ちると sounding が [60] になる。
    assert!(
        matches!(
            app.mml_overlay.line_status(),
            LineStatus::Played { note_count: 3, .. }
        ),
        "{:?}",
        app.mml_overlay.line_status()
    );
    assert!(app.mml_overlay.sounding().is_empty());
}

#[test]
fn t_on_the_chord_row_previews_what_the_generated_track_plays() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_generated_empty_cell();
    app.editor.cursor_track = 1;

    app.handle_normal_key_event(plain('t'));

    assert!(app.mml_overlay.is_patch_select_open());
    assert_eq!(
        app.mml_overlay.value(),
        crate::mml::cell_preview_line(&app.editor.data, 2, 1)
    );
    assert_eq!(app.mml_overlay.patch(), Some("Pads/Snapshot Pad.fxp"));
}

/// server の代わりに送った内容を記録する sender を app へ差す。
pub(in crate::mml_overlay_glue) fn attach_recording_sink(app: &mut DawApp) -> Arc<RecordingSink> {
    let sink = Arc::new(RecordingSink::default());
    app.mml_overlay_sender = Some(MmlOverlaySender::with_recording_sink(
        Arc::clone(&sink),
        48_000.0,
    ));
    sink
}

pub(in crate::mml_overlay_glue) fn wait_until(what: &str, mut done: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(5);
    while !done() {
        assert!(Instant::now() < deadline, "待ちきれなかった: {what}");
        std::thread::sleep(Duration::from_millis(2));
    }
}

#[test]
fn opening_plays_the_line_with_the_current_patch() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_generated_empty_cell();
    let sink = attach_recording_sink(&mut app);

    app.handle_normal_key_event(plain('t'));

    wait_until("開いた時点の試聴", || sink.timelines() >= 1);
    let patch = sink.prepared().last().cloned().expect("音色の準備");
    assert_eq!(patch.patch(), Some("Pads/Snapshot Pad.fxp"));
}
