//! 1 行モードの見た目。
//!
//! 入力欄の高さだけが複数行モードと違う。枠・タイトル・状態行は共通なので、
//! 「枠が何行あるか」と「状態行が枠の真下にあるか」を buffer から数えて判定する。

use super::*;

use crate::MmlOverlayInputMode;

fn single_line_overlay(initial_text: &str) -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        input_mode: MmlOverlayInputMode::SingleLine,
        initial_text: initial_text.to_string(),
        ..MmlOverlayContext::default()
    });
    overlay
}

/// 入力欄の枠が占める行数。上辺と下辺の行番号の差から数える。
fn input_box_height(rendered: &str) -> usize {
    let lines: Vec<&str> = rendered.lines().collect();
    let top = lines
        .iter()
        .position(|line| line.contains('┌'))
        .unwrap_or_else(|| panic!("枠の上辺が見つからない:\n{rendered}"));
    let bottom = lines
        .iter()
        .position(|line| line.contains('└'))
        .unwrap_or_else(|| panic!("枠の下辺が見つからない:\n{rendered}"));
    bottom - top + 1
}

#[test]
fn the_input_box_is_three_rows_tall_in_single_line_mode() {
    let rendered = render(&single_line_overlay("cde"));

    // 枠(2 行) + 入力 1 行。
    assert_eq!(input_box_height(&rendered), 3, "{rendered}");
}

#[test]
fn the_input_box_keeps_its_multi_line_height_in_multi_line_mode() {
    let rendered = render(&opened());

    // 枠(2 行) + 入力 8 行。従来のまま。
    assert_eq!(input_box_height(&rendered), 10, "{rendered}");
}

/// 状態行（`^T音色 ...`）は枠の真下の 1 行。1 行モードでも離れない。
#[test]
fn the_status_row_sits_right_below_the_box() {
    let rendered = render(&single_line_overlay("cde"));
    let lines: Vec<&str> = rendered.lines().collect();
    let bottom = lines.iter().position(|line| line.contains('└')).unwrap();

    assert!(
        lines[bottom + 1].contains("Esc"),
        "枠の真下が状態行のはず:\n{rendered}"
    );
}

#[test]
fn the_initial_text_is_drawn_inside_the_box() {
    let rendered = render(&single_line_overlay("cdefg"));

    assert!(rendered.contains("cdefg"), "{rendered}");
    assert!(rendered.contains("MML"), "{rendered}");
}

/// 空のときの案内は 1 行モード用の文言。複数行モードの「上下キーでその行を演奏」は出さない。
///
/// 全角文字は buffer 上でセルごとに分かれるので、続けて書いた 2 文字では照合できない
/// （既存の描画テストと同じ事情）。1 文字ずつで見る。
#[test]
fn an_empty_single_line_input_shows_its_own_placeholder() {
    let rendered = render(&single_line_overlay(""));

    assert!(rendered.contains("Enter"), "{rendered}");
    assert!(!rendered.contains('上'), "{rendered}");
}

#[test]
fn an_empty_multi_line_input_keeps_its_own_placeholder() {
    let rendered = render(&opened());

    assert!(rendered.contains('上'), "{rendered}");
    assert!(!rendered.contains("Enter"), "{rendered}");
}

/// 端末が低くても落ちない（高さが overlay より小さいときは切り詰める）。
#[test]
fn a_short_terminal_does_not_panic() {
    let overlay = single_line_overlay("cde");
    let mut terminal = ratatui::Terminal::new(TestBackend::new(80, 3)).unwrap();
    terminal
        .draw(|frame| {
            draw_with_status(&overlay, &MmlOverlaySenderStatus::default(), frame);
        })
        .unwrap();
}
