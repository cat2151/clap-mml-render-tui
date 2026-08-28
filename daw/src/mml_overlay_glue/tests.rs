//! DAW から開く MML 入力オーバーレイの配線の検証。
//!
//! 実際に音が出るかは play server を要するのでここでは見ない（引き継ぎ資料 §6）。
//! ここで押さえるのは「開く条件」「モード遷移」「オーバーレイへ渡した内容」
//! 「sender が無くても壊れない」の 4 つ。

use std::sync::Arc;
use std::time::Instant;

use cmrt_mml_overlay::{MmlOverlayAction, MmlOverlayInputMode};
use cmrt_runtime::Config;
use cmrt_tui_core::patch_load::PatchLoadState;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::{DawApp, DawMode, DawPlayState};
use crate::input::tests::build_test_app;

mod chord_transfer;
mod commit;
mod patch;

fn ctrl(code: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(code), KeyModifiers::CONTROL)
}

fn plain(code: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(code), KeyModifiers::NONE)
}

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn log_lines(app: &DawApp) -> Vec<String> {
    app.log_lines.lock().unwrap().iter().cloned().collect()
}

/// 走査経路と snapshot 経路を区別するための、実在しない patch dir を指す Config。
fn point_config_at_missing_patch_dir(app: &mut DawApp) {
    let missing = std::env::temp_dir().join("cmrt_test_daw_mml_overlay_missing_dir_absent");
    std::fs::remove_dir_all(&missing).ok();
    app.cfg = Arc::new(Config {
        patches_dirs: Some(vec![missing.to_string_lossy().into_owned()]),
        ..(*app.cfg).clone()
    });
}

fn snapshot_pairs() -> Vec<(String, String)> {
    vec![
        (
            "Bass/Snapshot Bass.fxp".to_string(),
            "bass/snapshot bass.fxp".to_string(),
        ),
        (
            "Pads/Snapshot Pad.fxp".to_string(),
            "pads/snapshot pad.fxp".to_string(),
        ),
    ]
}

#[test]
fn ctrl_p_opens_the_overlay_with_the_current_cell_in_a_single_line_input() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    app.editor.data[2][1] = "cdefg".to_string();

    assert!(app.try_open_mml_overlay(ctrl('p')));

    assert_eq!(app.mode, DawMode::MmlOverlay);
    assert!(app.mml_overlay.is_open());
    assert_eq!(
        app.mml_overlay.input_mode(),
        MmlOverlayInputMode::SingleLine
    );
    assert_eq!(app.mml_overlay.value(), "cdefg");
}

#[test]
fn a_key_other_than_ctrl_p_does_not_open_the_overlay() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_measure = 1;

    assert!(!app.try_open_mml_overlay(plain('p')));
    assert!(!app.try_open_mml_overlay(ctrl('t')));

    assert_eq!(app.mode, DawMode::Normal);
    assert!(!app.mml_overlay.is_open());
}

#[test]
fn the_init_column_does_not_open_the_overlay() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 0;
    app.editor.data[2][0] = r#"{"Surge XT patch": "Bass/Snapshot Bass.fxp"}"#.to_string();

    assert!(!app.try_open_mml_overlay(ctrl('p')));

    assert_eq!(app.mode, DawMode::Normal);
    assert!(!app.mml_overlay.is_open());
    assert!(
        log_lines(&app).iter().any(|line| line.contains("init 列")),
        "開かない理由をログに残すこと: {:?}",
        log_lines(&app)
    );
}

#[test]
fn a_mode_other_than_normal_does_not_open_the_overlay() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_measure = 1;
    app.mode = DawMode::Insert;

    assert!(!app.try_open_mml_overlay(ctrl('p')));

    assert_eq!(app.mode, DawMode::Insert);
    assert!(!app.mml_overlay.is_open());
}

#[test]
fn opening_the_overlay_stops_the_daw_playback() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_measure = 1;
    *app.playback.play_state.lock().unwrap() = DawPlayState::Playing;

    assert!(app.try_open_mml_overlay(ctrl('p')));

    assert!(
        *app.playback.play_state.lock().unwrap() == DawPlayState::Idle,
        "オーバーレイは音源 instance を借りるので、開く前に演奏を止めること"
    );
}

#[test]
fn the_overlay_opens_with_the_patch_of_the_cursor_track() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 3;
    app.editor.cursor_measure = 1;
    app.editor.data[2][0] = r#"{"Surge XT patch": "Pads/Snapshot Pad.fxp"}"#.to_string();
    app.editor.data[3][0] = r#"{"Surge XT patch": "Bass/Snapshot Bass.fxp"}"#.to_string();

    assert!(app.try_open_mml_overlay(ctrl('p')));

    assert_eq!(app.mml_overlay.patch(), Some("Bass/Snapshot Bass.fxp"));
}

#[test]
fn esc_closes_the_overlay_and_returns_to_normal() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_measure = 1;
    assert!(app.try_open_mml_overlay(ctrl('p')));

    app.handle_mml_overlay_key_event(key(KeyCode::Esc));

    assert_eq!(app.mode, DawMode::Normal);
    assert!(!app.mml_overlay.is_open());
}

#[test]
fn the_daw_keys_still_work_after_closing_the_overlay() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    assert!(app.try_open_mml_overlay(ctrl('p')));
    app.handle_mml_overlay_key_event(key(KeyCode::Esc));

    app.handle_normal_key_event(plain('h'));

    assert_eq!(app.editor.cursor_measure, 0);
}

/// 1 行モードなので `Enter` で改行が入ってはいけない。
/// 確定した中身とカーソルの動きは `commit` サブモジュールが見る。
#[test]
fn enter_never_inserts_a_newline() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_measure = 1;
    app.editor.data[2][1] = "cde".to_string();
    assert!(app.try_open_mml_overlay(ctrl('p')));

    app.handle_mml_overlay_key_event(key(KeyCode::Enter));

    assert_eq!(app.mode, DawMode::MmlOverlay);
    assert!(app.mml_overlay.is_open());
    assert!(!app.editor.data[2][1].contains('\n'));
    assert!(!app.mml_overlay.value().contains('\n'));
}

#[test]
fn typing_edits_the_line_and_asks_for_a_note_even_without_a_sender() {
    // sender が None（play server 無し）でも入力欄として成立すること。
    // 実際に音が出るかは play server が要るので §6 の確認リストへ回す。
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_measure = 1;
    assert!(app.mml_overlay_sender.is_none());
    assert!(app.try_open_mml_overlay(ctrl('p')));

    let action = app.mml_overlay.handle_key(plain('c'), Instant::now());

    assert!(
        matches!(action, MmlOverlayAction::Send(_)),
        "打鍵はその瞬間の音を求めるはず: {action:?}"
    );
    app.handle_mml_overlay_key_event(plain('d'));
    assert_eq!(app.mml_overlay.value(), "cd");
    app.handle_mml_overlay_key_event(key(KeyCode::Esc));
    assert_eq!(app.mode, DawMode::Normal);
}

#[test]
fn ctrl_t_opens_the_patch_select_from_the_injected_snapshot_without_scanning() {
    // 一覧は Stage 1 で注入した snapshot 由来。走査できない Config でも開けること。
    let (mut app, _cache_rx) = build_test_app();
    point_config_at_missing_patch_dir(&mut app);
    *app.patch_load.lock().unwrap() = PatchLoadState::ready(snapshot_pairs());
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    assert!(app.try_open_mml_overlay(ctrl('p')));

    app.handle_mml_overlay_key_event(ctrl('t'));

    assert!(
        app.mml_overlay.is_patch_select_open(),
        "snapshot が Ready なら、走査できない Config でも音色選択が開くはず"
    );
    assert!(!app.mml_overlay.is_waiting_for_patch_catalog());
}

#[test]
fn ctrl_t_waits_for_the_catalog_while_it_is_still_loading() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    app.editor.cursor_measure = 1;
    assert!(matches!(
        *app.patch_load.lock().unwrap(),
        PatchLoadState::Loading
    ));
    assert!(app.try_open_mml_overlay(ctrl('p')));

    app.handle_mml_overlay_key_event(ctrl('t'));
    assert!(!app.mml_overlay.is_patch_select_open());
    assert!(app.mml_overlay.is_waiting_for_patch_catalog());

    // loader が完了したら、毎フレームの pump が一覧を差し替えて予約を実行する。
    *app.patch_load.lock().unwrap() = PatchLoadState::ready(snapshot_pairs());
    app.pump_mml_overlay();

    assert!(
        app.mml_overlay.is_patch_select_open(),
        "Loading 中の Ctrl+T は、一覧が来た時点で開くこと"
    );
}

/// chord 行の中身はコード進行なので、MML として鳴らすオーバーレイでは開かない。
/// `i` はインラインの INSERT へ落ちて、そのまま文字を編集できる。
#[test]
fn the_chord_row_falls_back_to_the_inline_insert_instead_of_the_overlay() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = crate::CHORD_TRACK;
    app.editor.cursor_measure = 1;
    app.editor.data[crate::CHORD_TRACK][1] = "I-IV-V-I".to_string();

    app.open_mml_overlay_or_insert();

    assert_eq!(app.mode, DawMode::Insert);
    assert!(!app.mml_overlay.is_open());
    assert_eq!(app.textarea.lines().join("\n"), "I-IV-V-I");
}
