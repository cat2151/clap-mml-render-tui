//! `i` の行き先。MML 入力はオーバーレイへ寄せ、
//! init 列（meas 0）だけが従来のインライン INSERT に残る。
//!
//! init セルの中身は音色 JSON なので、1 行 MML として上書きさせると
//! 音色指定が壊れる。その直接編集手段を残すための例外。

use super::*;

use cmrt_mml_overlay::MmlOverlayInputMode;

#[test]
fn handle_normal_i_opens_the_mml_overlay() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 1;
    app.editor.data[1][1] = "l8cdefg".to_string();

    let result = app.handle_normal(KeyCode::Char('i'));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.mode, DawMode::MmlOverlay);
    assert!(app.mml_overlay.is_open());
    assert_eq!(
        app.mml_overlay.input_mode(),
        MmlOverlayInputMode::SingleLine
    );
    assert_eq!(app.mml_overlay.value(), "l8cdefg");
}

#[test]
fn handle_normal_i_on_the_init_column_still_starts_the_inline_insert() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 0;
    app.editor.data[1][0] = r#"{"Surge XT patch": "Bass/Snapshot Bass.fxp"}"#.to_string();

    app.handle_normal(KeyCode::Char('i'));

    assert_eq!(app.mode, DawMode::Insert);
    assert!(!app.mml_overlay.is_open());
    assert_eq!(
        app.textarea.lines().join("\n"),
        r#"{"Surge XT patch": "Bass/Snapshot Bass.fxp"}"#,
        "init セルは従来どおり生の JSON を直接編集する"
    );
}

/// init 列の `i` は「開けなかった」のではなく「インラインで開いた」のだから、
/// `Ctrl+P` のときの理由ログを出してはいけない。
#[test]
fn handle_normal_i_on_the_init_column_does_not_log_the_overlay_refusal() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 0;

    app.handle_normal(KeyCode::Char('i'));

    let logs: Vec<String> = app.log_lines.lock().unwrap().iter().cloned().collect();
    assert!(
        !logs.iter().any(|line| line.contains("init 列")),
        "インラインで開けているのに断りのログを出さないこと: {logs:?}"
    );
}

/// `i` と `Ctrl+P` は同じ入口。片方だけが古い経路に残らないことを見る。
#[test]
fn i_and_ctrl_p_open_the_same_overlay() {
    let (mut by_i, _rx_i) = build_test_app();
    by_i.editor.cursor_track = 1;
    by_i.editor.cursor_measure = 1;
    by_i.editor.data[1][1] = "o5c".to_string();
    by_i.handle_normal(KeyCode::Char('i'));

    let (mut by_ctrl_p, _rx_p) = build_test_app();
    by_ctrl_p.editor.cursor_track = 1;
    by_ctrl_p.editor.cursor_measure = 1;
    by_ctrl_p.editor.data[1][1] = "o5c".to_string();
    assert!(by_ctrl_p.try_open_mml_overlay(KeyEvent::new(
        KeyCode::Char('p'),
        crossterm::event::KeyModifiers::CONTROL,
    )));

    assert_eq!(by_i.mode, by_ctrl_p.mode);
    assert_eq!(by_i.mml_overlay.value(), by_ctrl_p.mml_overlay.value());
    assert_eq!(
        by_i.mml_overlay.input_mode(),
        by_ctrl_p.mml_overlay.input_mode()
    );
}
