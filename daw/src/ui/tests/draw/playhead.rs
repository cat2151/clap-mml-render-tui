//! meas ヘッダ行の演奏位置表示（playhead）の描画テスト。

use super::super::*;
use super::grid::{header_row, x_of_in_row};
use cmrt_tui_core::theme::{MONOKAI_CURSOR_BG, MONOKAI_YELLOW};

/// 4/4・1 小節 4 秒として、`elapsed` だけ経過した状態を作る。
fn start_playing(app: &DawApp, state: DawPlayState, measure_index: usize, elapsed_secs: f64) {
    *app.playback.play_state.lock().unwrap() = state;
    *app.playback.position.lock().unwrap() = Some(PlayPosition {
        measure_index,
        measure_start: std::time::Instant::now()
            .checked_sub(std::time::Duration::from_secs_f64(elapsed_secs))
            .unwrap(),
        measure_duration: std::time::Duration::from_secs(4),
    });
}

/// ヘッダ行の x から始まる `count` セルの背景色。
fn header_backgrounds(buffer: &Buffer, x: u16, count: u16) -> Vec<Color> {
    let y = header_row(buffer);
    (0..count)
        .map(|offset| buffer.cell((x + offset, y)).unwrap().bg)
        .collect()
}

/// 停止中は従来どおり `M2`。背景も敷かない。
#[test]
fn idle_header_keeps_the_plain_measure_label() {
    let buffer = render_buffer(&build_test_app(), 60, 19);
    let header_y = header_row(&buffer);

    let x = x_of_in_row(&buffer, header_y, "M2");
    for bg in header_backgrounds(&buffer, x, 9) {
        assert_ne!(bg, MONOKAI_YELLOW, "idle header must not be painted");
        assert_ne!(bg, MONOKAI_CURSOR_BG, "idle header must not be painted");
    }
}

/// 演奏中の小節だけラベルが `>2` になる。他の小節は `M1` のまま。
#[test]
fn playing_measure_label_switches_to_the_arrow() {
    let app = build_test_app();
    start_playing(&app, DawPlayState::Playing, 1, 0.0);

    let buffer = render_buffer(&app, 60, 19);
    let header_y = header_row(&buffer);
    let row: String = (0..buffer.area.width)
        .map(|x| buffer.cell((x, header_y)).unwrap().symbol().to_string())
        .collect();

    assert!(row.contains(">2"), "header: {row:?}");
    assert!(row.contains("M1"), "header: {row:?}");
    assert!(!row.contains("M2"), "header: {row:?}");
}

/// playhead が出ても列は 1 桁もずれない。
#[test]
fn playhead_does_not_shift_the_measure_columns() {
    let app = build_test_app();
    start_playing(&app, DawPlayState::Playing, 1, 0.0);

    let buffer = render_buffer(&app, 60, 19);
    let header_y = header_row(&buffer);

    let init_x = x_of_in_row(&buffer, header_y, "Init");
    let m1_x = x_of_in_row(&buffer, header_y, "M1");
    let playing_x = x_of_in_row(&buffer, header_y, ">2");

    assert_eq!(m1_x - init_x, 14, "init column stays 14 columns wide");
    assert_eq!(
        playing_x - m1_x,
        9,
        "measure columns use the available width"
    );
}

/// 8 桁セルの 1 拍目は 2 桁塗り、残り 6 桁は暗い背景。区切り空白は塗らない。
#[test]
fn first_beat_paints_the_first_quarter_of_the_cell() {
    let app = build_test_app();
    start_playing(&app, DawPlayState::Playing, 1, 0.0);

    let buffer = render_buffer(&app, 60, 19);
    let header_y = header_row(&buffer);
    let x = x_of_in_row(&buffer, header_y, ">2");

    let backgrounds = header_backgrounds(&buffer, x, 9);
    assert_eq!(backgrounds[0], MONOKAI_YELLOW);
    assert_eq!(backgrounds[1], MONOKAI_YELLOW);
    assert_eq!(backgrounds[2], MONOKAI_CURSOR_BG);
    assert_eq!(backgrounds[3], MONOKAI_CURSOR_BG);
    assert_ne!(backgrounds[8], MONOKAI_YELLOW, "column gap stays unpainted");
    assert_ne!(
        backgrounds[8], MONOKAI_CURSOR_BG,
        "column gap stays unpainted"
    );
}

/// 拍が進むと 8 桁セルの塗りが 2 桁ずつ右へ伸びる。
#[test]
fn fill_grows_one_quarter_per_beat() {
    for (elapsed_secs, expected_filled) in [(0.0, 2), (1.0, 4), (2.0, 6), (3.0, 8)] {
        let app = build_test_app();
        start_playing(&app, DawPlayState::Playing, 1, elapsed_secs);

        let buffer = render_buffer(&app, 60, 19);
        let header_y = header_row(&buffer);
        let x = x_of_in_row(&buffer, header_y, ">2");
        let filled = header_backgrounds(&buffer, x, 8)
            .iter()
            .filter(|bg| **bg == MONOKAI_YELLOW)
            .count();

        assert_eq!(filled, expected_filled, "elapsed={elapsed_secs}");
    }
}

/// preview でも同じ場所に出る。色だけが変わる。
#[test]
fn preview_paints_the_header_in_its_own_color() {
    let app = build_test_app();
    start_playing(&app, DawPlayState::Preview, 1, 0.0);

    let buffer = render_buffer(&app, 60, 19);
    let header_y = header_row(&buffer);
    let x = x_of_in_row(&buffer, header_y, ">2");

    let backgrounds = header_backgrounds(&buffer, x, 8);
    assert_eq!(backgrounds[0], MONOKAI_PURPLE);
    assert_eq!(backgrounds[1], MONOKAI_PURPLE);
    assert_eq!(backgrounds[2], MONOKAI_CURSOR_BG);
}

/// A-B マーカーの小節を演奏中でも、A / B のラベルと色は残る。
#[test]
fn ab_marker_keeps_its_label_and_color_under_the_playhead() {
    let app = build_test_app();
    {
        let mut ab_repeat = app.playback.ab_repeat.lock().unwrap();
        *ab_repeat = AbRepeatState::FixEnd {
            start_measure_index: 0,
            end_measure_index: 1,
        };
    }
    start_playing(&app, DawPlayState::Playing, 1, 0.0);

    let buffer = render_buffer(&app, 60, 19);
    let header_y = header_row(&buffer);
    let x = x_of_in_row(&buffer, header_y, "B2");

    // 8 桁セルでは 1 拍目にラベル 2 桁を塗り終える。残りは B マーカーの色（紫）のまま。
    let unfilled = buffer.cell((x + 2, header_y)).unwrap();
    assert_eq!(unfilled.symbol(), " ");
    assert_eq!(unfilled.fg, MONOKAI_PURPLE);
    assert_eq!(unfilled.bg, MONOKAI_CURSOR_BG);
    // 塗った桁は反転している。
    assert_eq!(buffer.cell((x, header_y)).unwrap().bg, MONOKAI_YELLOW);
    assert_eq!(buffer.cell((x + 1, header_y)).unwrap().bg, MONOKAI_YELLOW);
}
