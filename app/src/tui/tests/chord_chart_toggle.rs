//! `Shift+P` / `Space` のトグル（3.3）が glue まで届くこと。
//!
//! **「鳴っているか」を知っているのは glue だけ**（`MmlOverlaySenderStatus::sounding()`
//! は打鍵の生 MIDI 専用で、行の演奏では空のまま。2026-09-09 に
//! `mml-overlay/src/sender/tests.rs` の
//! `a_line_performance_leaves_the_sounding_status_empty` で実測）。ここで見るのは、
//! glue が持つ「鳴り終わる時刻」が画面へ正しく写り、トグルの向きを変えること。
//!
//! テストの app は `mml_overlay_sender` が `None`（音は出ない）なので、
//! 「鳴っている最中」は glue が持つ時刻を直に置いて作る。

use super::chord_chart_preview::app_on_the_chord_chart;
use super::*;
use crate::tui::chord_chart_glue::preview_duration;
use cmrt_chord_chart::PreviewRequest;
use cmrt_mml_overlay::line_play::{LinePerformance, LineProgram};
use std::time::{Duration, Instant};

fn space() -> KeyEvent {
    KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE)
}

fn shift_p() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('P'), KeyModifiers::SHIFT)
}

/// 1 回きりの演奏なので、**鳴り終わる時刻を過ぎたら鳴っていない**。
#[test]
fn the_preview_stops_being_sounding_once_its_last_event_has_passed() {
    let mut app = app_on_the_chord_chart();
    let now = Instant::now();
    assert!(
        !app.chord_chart_preview_sounding(now),
        "何も投げていなければ鳴っていない"
    );

    app.chord_chart_preview_ends_at = Some(now + Duration::from_secs(8));

    assert!(app.chord_chart_preview_sounding(now));
    assert!(app.chord_chart_preview_sounding(now + Duration::from_secs(7)));
    assert!(
        !app.chord_chart_preview_sounding(now + Duration::from_secs(9)),
        "鳴り終わったあとに `Space` が「止める」へ化けないこと"
    );
}

/// 何秒鳴るかは、送るイベント列そのものから測る。
///
/// 2026-09-09 実測: 既定 prefix の `I-V-VIm-IV` は 4 和音 × 2 秒 = 8 秒
/// （最後の note off が 8.0 秒）。
#[test]
fn the_length_of_a_preview_is_measured_from_the_events_it_sends() {
    let app = app_on_the_chord_chart();

    let played = app.chord_chart_preview(&PreviewRequest {
        name: "A".to_string(),
        degrees: "I-V-VIm-IV".to_string(),
        chord_index: None,
    });

    assert_eq!(
        preview_duration(&played.program),
        Some(Duration::from_secs_f64(8.0))
    );
    assert_eq!(
        preview_duration(&LineProgram::silent()),
        None,
        "止めるだけの指示は「鳴っている」を作らないこと"
    );
    // 壊れた値でも panic しない（`Duration::from_secs_f64` は負で落ちる）。
    assert_eq!(
        preview_duration(&LineProgram::once(LinePerformance {
            events: Vec::new(),
            loop_seconds: -1.0,
        })),
        None
    );
}

/// glue が「鳴っている」の答えを画面へ書き戻す。画面はこれを読んでトグルの向きを決める。
#[test]
fn the_glue_writes_the_sounding_answer_into_the_screen() {
    let mut app = app_on_the_chord_chart();
    assert!(!app.chord_chart.preview_sounding());

    app.chord_chart_preview_ends_at = Some(Instant::now() + Duration::from_secs(8));
    // キーを渡す前に書き戻すので、どのキーでも更新される（`k` は端で何も起こさない）。
    app.handle_chord_chart_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    assert!(app.chord_chart.preview_sounding());

    app.chord_chart_preview_ends_at = Some(Instant::now() - Duration::from_secs(1));
    app.handle_chord_chart_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    assert!(
        !app.chord_chart.preview_sounding(),
        "鳴り終わっていたら画面にもそう伝わること"
    );
}

/// 鳴っている最中のトグルは**止める要求**になり、その場で glue が回収する。
///
/// 回収されたあとの画面からは要求が見えないので、同じ状態の複製へ同じキーを打つ
/// 対照実験で「何が立ったか」を見る（`chord_chart_preview` と同じ作法）。
#[test]
fn the_toggle_asks_for_silence_while_the_preview_is_sounding() {
    for press in [space(), shift_p()] {
        let mut app = app_on_the_chord_chart();
        app.chord_chart_preview_ends_at = Some(Instant::now() + Duration::from_secs(8));
        // 画面へ「鳴っている」を書き戻させるための 1 打（端の `k` は何も起こさない）。
        app.handle_chord_chart_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));

        let mut unglued = app.chord_chart.clone();
        unglued.handle_key_event(press);
        assert_eq!(
            unglued.take_preview(),
            Some(PreviewRequest::silent()),
            "{press:?} は止める要求を立てること（対照）"
        );

        app.handle_chord_chart_key_event(press);

        assert_eq!(
            app.chord_chart.take_preview(),
            None,
            "glue が回収して残さないこと"
        );
    }
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
    app.chord_chart_preview_ends_at = Some(Instant::now() + Duration::from_secs(8));

    assert!(app.try_open_mml_overlay(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL)));

    assert!(app.mml_overlay.is_open());
    assert!(!app.chord_chart_preview_sounding(Instant::now()));
    assert!(!app.chord_chart.preview_sounding());
}

/// 1 行入力欄が開いている間、`Space` は**文字**として入る（トグルにならない）。
///
/// 画面 crate 側は `song` で見ている（`screen::preview::tests::toggle`）。ここでは
/// **buffer に出た文字**まで確かめる（受け入れ条件どおり。song に入っていても
/// 描けていなければ打った本人には分からない）。
#[test]
fn a_space_typed_into_the_line_input_reaches_the_screen_as_a_character() {
    use super::chord_chart_preview::{chord_chart_rows, sections_pane_rows};

    let mut app = app_on_the_chord_chart();
    app.handle_chord_chart_key_event(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE));
    for _ in 0..64 {
        app.handle_chord_chart_key_event(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
    }
    for code in ['I', ' ', 'V'] {
        app.handle_chord_chart_key_event(KeyEvent::new(KeyCode::Char(code), KeyModifiers::NONE));
    }
    // 確定する前から、打った空白は入力欄に見えている。
    let typing = chord_chart_rows(&app);
    assert!(
        typing.iter().any(|row| row.contains("I V")),
        "入力欄に打った空白が見えること: {typing:?}"
    );

    app.handle_chord_chart_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    let pane_rows = sections_pane_rows(&chord_chart_rows(&app));
    assert!(
        pane_rows.iter().any(|row| row.contains("I V")),
        "確定した進行が section の行に出ること: {pane_rows:?}"
    );
    assert_eq!(app.chord_chart.song.sections[0].degrees, "I V");
}
