//! `Shift+P` / `Space` のトグルが glue まで届くこと。
//!
//! sender が実際に公開した line 区間と、Chord Chart が送った command id の一致だけが
//! トグルを「止める」向きにする。enqueue 時刻からの推定は使わない。

use super::chord_chart_preview::app_on_the_chord_chart;
use super::*;
use crate::tui::chord_chart_glue::preview_command_sounding;
use cmrt_chord_chart::PreviewRequest;
use std::time::Instant;

fn space() -> KeyEvent {
    KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)
}

fn shift_p() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT)
}

/// 遅い patch load / timeline 送信中は区間がまだ無い。実際に積み終えた matching
/// command の区間が鳴り始めたときだけ Stop 側へ切り替わる。
#[test]
fn slow_load_becomes_sounding_only_after_the_matching_line_starts() {
    let expected = Some(41);

    assert!(!preview_command_sounding(expected, None), "load 中");
    assert!(preview_command_sounding(expected, Some((41, true))));
}

/// 停止、別 command による supersede、sender 区間の expiry はすべて Play 側へ戻す。
#[test]
fn stop_supersession_and_expiry_are_not_sounding() {
    let expected = Some(41);

    assert!(!preview_command_sounding(expected, None), "Stop 後");
    assert!(
        !preview_command_sounding(expected, Some((42, true))),
        "別 command に supersede 済み"
    );
    assert!(
        !preview_command_sounding(expected, Some((41, false))),
        "matching 区間も終了時刻以後は silent"
    );
    assert!(!preview_command_sounding(None, Some((41, true))));
}

/// sender が無い状態では、古い画面側フラグもキーを渡す前に false へ戻す。
#[test]
fn the_glue_clears_the_screen_answer_without_a_sender() {
    let mut app = app_on_the_chord_chart();
    app.chord_chart_preview_command_id = Some(41);
    app.chord_chart.set_preview_sounding(true);
    app.handle_chord_chart_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    assert!(!app.chord_chart.preview_sounding());
}

/// 止まっているときのトグルは**カーソル行を鳴らす要求**になる。
#[test]
fn the_toggle_asks_to_play_the_cursor_section_while_silent() {
    for press in [space(), shift_p()] {
        let mut app = app_on_the_chord_chart();
        assert!(!app.chord_chart_preview_sounding(Instant::now()));

        let mut unglued = app.chord_chart.clone();
        unglued.handle_key_event(press);
        assert_eq!(
            unglued.take_preview(),
            Some(PreviewRequest {
                name: "A".to_string(),
                degrees: "I-V-VIm-IV".to_string(),
                chord_index: None,
            }),
            "{press:?} は鳴らす要求を立てること（対照）"
        );

        app.handle_chord_chart_key_event(press);

        assert_eq!(app.chord_chart.take_preview(), None);
        assert_eq!(app.chord_chart.error, None);
    }
}

/// play server が上がっていない（sender が `None`）なら、何を押しても
/// 「鳴っている」にはならない。押しても落ちない。
#[test]
fn without_a_sender_the_toggle_never_believes_it_is_sounding() {
    let mut app = app_on_the_chord_chart();
    assert!(app.mml_overlay_sender.is_none());

    app.handle_chord_chart_key_event(space());
    app.handle_chord_chart_key_event(shift_p());
    app.handle_chord_chart_key_event(space());

    assert!(!app.chord_chart_preview_sounding(Instant::now()));
    assert!(!app.chord_chart.preview_sounding());
    assert_eq!(app.chord_chart.error, None);
}

/// MML オーバーレイ（`Ctrl+P`）へ音源を明け渡したら、「鳴っている」の記録も捨てる。
///
/// 音そのものは相手が止める（`sender.prepare()` の中の `voice.stop`。
/// `mml-overlay/src/sender/tests.rs` の
/// `preparing_an_already_ready_patch_stops_the_previous_line` が固定）。
/// 記録だけ残すと、閉じて戻ったあとの `Space` が空打ちになる。
#[test]
fn handing_the_instrument_to_the_mml_overlay_forgets_the_sounding_preview() {
    let mut app = app_on_the_chord_chart();
    app.chord_chart_preview_command_id = Some(41);
    app.chord_chart.set_preview_sounding(true);

    assert!(app.try_open_mml_overlay(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL)));

    assert!(app.mml_overlay.is_open());
    assert_eq!(app.chord_chart_preview_command_id, None);
    assert!(!app.chord_chart_preview_sounding(Instant::now()));
    assert!(!app.chord_chart.preview_sounding());
}

/// MML overlay が開いている間、`Space` は**文字**として入る（トグルにならない）。
///
/// ここでは **buffer に出た文字**まで確かめる（overlay の値に入っていても
/// 描けていなければ打った本人には分からない）。
#[test]
fn a_space_typed_into_the_mml_overlay_reaches_the_screen_as_a_character() {
    use super::chord_chart_preview::{chord_chart_rows, sections_pane_rows};

    let mut app = app_on_the_chord_chart();
    app.handle_chord_chart_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    for _ in 0..64 {
        app.handle_mml_overlay_key_event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    }
    for code in ['I', ' ', 'V'] {
        app.handle_mml_overlay_key_event(KeyEvent::new(KeyCode::Char(code), KeyModifiers::NONE));
    }
    // 確定する前から、打った空白は入力欄に見えている。
    let typing = crate::tui::ui::tests::render_lines(&mut app, 80, 24);
    assert!(
        typing.iter().any(|row| row.contains("I V")),
        "入力欄に打った空白が見えること: {typing:?}"
    );

    app.handle_mml_overlay_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let pane_rows = sections_pane_rows(&chord_chart_rows(&app));
    assert!(
        pane_rows.iter().any(|row| row.contains("I V")),
        "確定した進行が section の行に出ること: {pane_rows:?}"
    );
    assert_eq!(app.chord_chart.song.sections[0].degrees, "I V");
}
