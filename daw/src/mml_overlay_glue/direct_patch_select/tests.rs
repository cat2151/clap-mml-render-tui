//! NORMAL の `t` から音色 selector だけを開く経路の検証。
//!
//! guard の握り方は `mml_overlay_glue/tests.rs` の注意書きと同じ（1 テスト = 1 guard、
//! テスト関数の先頭）。

use std::sync::Arc;
use std::time::{Duration, Instant};

use cmrt_mml_overlay::line_play::line_events;
use cmrt_mml_overlay::{MmlOverlaySender, PatchAudition, RecordingSink};
use cmrt_tui_core::patch_load::PatchLoadState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::super::{DawApp, DawMode};
use crate::input::tests::build_test_app;

const PAD_INIT_CELL: &str = r#"{"Surge XT patch": "Pads/Snapshot Pad.fxp"}"#;
const KEYS_INIT_CELL: &str = r#"{"Surge XT patch": "Keys/Snapshot Keys.fxp"}"#;

pub(in crate::mml_overlay_glue) fn plain(code: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(code), KeyModifiers::NONE)
}

pub(in crate::mml_overlay_glue) fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// 2 つとも Chord 用途にして、pad の track で開いた selector（Chord で開く）の中で行き来できるようにする。
fn catalog_pairs() -> Vec<(String, String)> {
    ["Keys/Snapshot Keys.fxp", "Pads/Snapshot Pad.fxp"]
        .into_iter()
        .map(|display| (display.to_string(), display.to_lowercase()))
        .collect()
}

/// `t` の selector が開いている（一覧の Loading 待ちではない）。
pub(in crate::mml_overlay_glue) fn selector_is_open(app: &DawApp) -> bool {
    app.direct_patch_select
        .as_ref()
        .is_some_and(|select| select.is_select_open())
}

fn selector_patch(app: &DawApp) -> Option<&str> {
    app.direct_patch_select.as_ref()?.patch()
}

/// `t` が試聴に渡したもの。
fn audition(app: &DawApp) -> Option<&PatchAudition> {
    app.direct_patch_select.as_ref()?.audition()
}

/// その MML を行として試聴する、という試聴。
fn line_audition(line: &str) -> PatchAudition {
    PatchAudition::Line(line_events(line).1)
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

    assert_eq!(app.mode, DawMode::DirectPatchSelect);
    assert!(selector_is_open(&app));
    // MML 入力欄は開かない。
    assert!(!app.mml_overlay.is_open());
    assert_eq!(selector_patch(&app), Some("Pads/Snapshot Pad.fxp"));
}

#[test]
fn confirming_writes_the_init_cell_and_returns_to_normal() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_pad_track();
    app.handle_normal_key_event(plain('t'));

    app.handle_direct_patch_select_key_event(plain('/'));
    for ch in "snapshot keys".chars() {
        app.handle_direct_patch_select_key_event(plain(ch));
    }
    // 1 回目は絞り込み、2 回目は音色の確定。
    app.handle_direct_patch_select_key_event(key(KeyCode::Enter));
    app.handle_direct_patch_select_key_event(key(KeyCode::Enter));

    assert_eq!(app.editor.data[2][0], KEYS_INIT_CELL);
    assert_eq!(app.mode, DawMode::Normal);
    assert!(app.direct_patch_select.is_none());
    assert!(!app.mml_overlay.is_open());
    // meas のセルには触れないこと。
    assert_eq!(app.editor.data[2][1], "cde");
    assert_eq!(app.editor.cursor_measure, 1);
    assert_eq!(*app.playback.auto_play_reservation.lock().unwrap(), Some(1));
}

#[test]
fn cancelling_keeps_the_init_cell_and_returns_to_normal() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_pad_track();
    app.handle_normal_key_event(plain('t'));

    app.handle_direct_patch_select_key_event(key(KeyCode::Esc));

    assert_eq!(app.editor.data[2][0], PAD_INIT_CELL);
    assert_eq!(app.mode, DawMode::Normal);
    assert!(!app.mml_overlay.is_open());
    assert_eq!(*app.playback.auto_play_reservation.lock().unwrap(), None);
}

#[test]
fn the_init_column_opens_the_selector_with_the_single_preview_note() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_pad_track();
    app.editor.cursor_measure = 0;

    app.handle_normal_key_event(plain('t'));

    assert!(selector_is_open(&app));
    assert!(
        matches!(audition(&app), Some(PatchAudition::Notes(_))),
        "{:?}",
        audition(&app)
    );
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
    assert_eq!(app.mode, DawMode::DirectPatchSelect);
    assert!(app
        .direct_patch_select
        .as_ref()
        .is_some_and(|select| select.is_waiting_for_catalog()));

    // 待っている間の文字キーは何もしない。
    app.handle_direct_patch_select_key_event(plain('x'));
    assert_eq!(app.mode, DawMode::DirectPatchSelect);

    app.handle_direct_patch_select_key_event(key(KeyCode::Esc));
    assert_eq!(app.mode, DawMode::Normal);
    assert!(app.direct_patch_select.is_none());
    assert_eq!(app.editor.data[2][1], "cde");
}

#[test]
fn the_catalog_arriving_while_waiting_opens_the_selector() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_pad_track();
    *app.patch_load.lock().unwrap() = PatchLoadState::Loading;
    app.handle_normal_key_event(plain('t'));

    *app.patch_load.lock().unwrap() = PatchLoadState::ready(catalog_pairs());
    app.pump_mml_overlay();

    assert!(selector_is_open(&app));
}

#[test]
fn an_empty_catalog_does_not_leave_the_overlay_open() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_pad_track();
    *app.patch_load.lock().unwrap() = PatchLoadState::ready(Vec::new());

    app.handle_normal_key_event(plain('t'));

    assert_eq!(app.mode, DawMode::Normal);
    assert!(!app.mml_overlay.is_open());
    assert!(app.direct_patch_select.is_none());
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
    assert!(selector_is_open(&app));
    assert_eq!(audition(&app), Some(&line_audition(&expected)));
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
fn t_on_the_chord_row_previews_what_the_generated_track_plays() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_generated_empty_cell();
    app.editor.cursor_track = 1;

    app.handle_normal_key_event(plain('t'));

    assert!(selector_is_open(&app));
    assert_eq!(
        audition(&app),
        Some(&line_audition(&crate::mml::cell_preview_line(
            &app.editor.data,
            2,
            1
        )))
    );
    assert_eq!(selector_patch(&app), Some("Pads/Snapshot Pad.fxp"));
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

#[test]
fn the_preview_goes_through_the_track_effect_chain() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_pad_track();
    app.editor.data[2][0] = r#"{"Surge XT patch": "Pads/Snapshot Pad.fxp", "effects after instrument": [{"Surge XT Effects preset": "Reverb 1/Cathedral 2.srgfx"}]}"#.to_string();
    let sink = attach_recording_sink(&mut app);

    app.handle_normal_key_event(plain('t'));
    app.handle_direct_patch_select_key_event(key(KeyCode::Up));

    wait_until("前の候補の試聴", || {
        sink.prepared()
            .iter()
            .any(|patch| patch.patch() == Some("Keys/Snapshot Keys.fxp"))
    });
    let prepared = sink.prepared();
    assert!(!prepared.is_empty());
    for patch in prepared {
        assert_eq!(
            patch.effect_chain(),
            r#"[{"Surge XT Effects preset":"Reverb 1/Cathedral 2.srgfx"}]"#
        );
    }
}

fn app_with_drum_catalog(init_cell: &str) -> (DawApp, std::sync::mpsc::Receiver<crate::CacheJob>) {
    let (mut app, cache_rx) = build_test_app();
    let pairs = [
        "Drums/Kick A.fxp",
        "Drums/Kick B.fxp",
        "Drums/Snare Z.fxp",
        "Pads/Snapshot Pad.fxp",
    ]
    .into_iter()
    .map(|display| (display.to_string(), display.to_lowercase()))
    .collect();
    *app.patch_load.lock().unwrap() = PatchLoadState::ready(pairs);
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    app.editor.data[2][0] = init_cell.to_string();
    app.editor.data[2][1] = "c".to_string();
    (app, cache_rx)
}

/// `End` で一覧の末尾を確定し、開いた直後の候補がどこまで絞られていたかを見る。
fn confirm_last_candidate(app: &mut DawApp) {
    app.handle_normal_key_event(plain('t'));
    app.handle_direct_patch_select_key_event(key(KeyCode::End));
    app.handle_direct_patch_select_key_event(key(KeyCode::Enter));
}

#[test]
fn a_kick_track_opens_the_selector_on_the_kick_preset() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_drum_catalog(r#"{"Surge XT patch": "Drums/Kick A.fxp"}"#);

    confirm_last_candidate(&mut app);

    assert!(
        app.editor.data[2][0].contains("Drums/Kick B.fxp"),
        "{}",
        app.editor.data[2][0]
    );
}

#[test]
fn a_track_without_a_patch_opens_the_selector_on_all() {
    let (_temp, _env_guard) = crate::input::tests::temp_local_dirs("mml_overlay");
    let (mut app, _cache_rx) = app_with_drum_catalog("");

    confirm_last_candidate(&mut app);

    assert!(
        app.editor.data[2][0].contains("Pads/Snapshot Pad.fxp"),
        "{}",
        app.editor.data[2][0]
    );
}

mod audition;
