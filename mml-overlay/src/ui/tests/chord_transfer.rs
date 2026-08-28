//! chord ヒント行と確定ダイアログの見た目。
//!
//! 全角文字は buffer 上でセルごとに分かれるので、続けて書いた 2 文字では照合できない
//! （既存の描画テストと同じ事情）。ASCII の断片か 1 文字ずつで見る。

use super::*;

use crate::MmlOverlayInputMode;

fn daw_overlay(initial_text: &str) -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        input_mode: MmlOverlayInputMode::SingleLine,
        initial_text: initial_text.to_string(),
        chord_row_transfer: true,
        ..MmlOverlayContext::default()
    });
    overlay
}

/// 入力欄の枠の下辺の行番号。
fn box_bottom(rendered: &str) -> usize {
    rendered
        .lines()
        .position(|line| line.contains('└'))
        .unwrap_or_else(|| panic!("枠の下辺が見つからない:\n{rendered}"))
}

#[test]
fn a_chord_notation_adds_a_hint_row_under_the_status_row() {
    let rendered = render(&daw_overlay("I-IV"));
    let lines: Vec<&str> = rendered.lines().collect();
    let bottom = box_bottom(&rendered);

    // 枠の真下は従来どおり状態行。
    assert!(lines[bottom + 1].contains("Esc"), "{rendered}");
    // その 1 行下がヒント。
    assert!(lines[bottom + 2].contains("chord"), "{rendered}");
    assert!(lines[bottom + 2].contains("Enter"), "{rendered}");
}

/// ヒントが立っていないときは行が増えない（overlay の高さが戻る）。
#[test]
fn plain_mml_leaves_the_overlay_the_same_height_as_before() {
    let rendered = render(&daw_overlay("cdefg"));
    let lines: Vec<&str> = rendered.lines().collect();
    let bottom = box_bottom(&rendered);

    assert!(lines[bottom + 1].contains("Esc"), "{rendered}");
    assert!(
        lines[bottom + 2].trim().is_empty(),
        "ヒント行が残っている:\n{rendered}"
    );
}

/// 移送先の無い画面（notepad / keyboard / grid）では出ない。
#[test]
fn a_screen_without_a_chord_row_draws_no_hint() {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        input_mode: MmlOverlayInputMode::SingleLine,
        initial_text: "I-IV".to_string(),
        ..MmlOverlayContext::default()
    });
    let rendered = render(&overlay);
    let bottom = box_bottom(&rendered);
    let lines: Vec<&str> = rendered.lines().collect();

    assert!(
        lines[bottom + 2].trim().is_empty(),
        "移送先が無いのにヒントが出ている:\n{rendered}"
    );
}

#[test]
fn the_confirm_dialog_shows_both_choices_over_the_input() {
    let mut overlay = daw_overlay("I-IV");
    overlay.handle_key(
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        Instant::now(),
    );

    let rendered = render(&overlay);

    assert!(rendered.contains("chord"), "{rendered}");
    assert!(rendered.contains("MML"), "{rendered}");
    assert!(rendered.contains("Enter"), "{rendered}");
    assert!(rendered.contains("Esc"), "{rendered}");
}

/// 既定の選択行（移送）が塗られ、もう一方は塗られていないこと。
///
/// **入力欄の枠を掴まないこと。** ダイアログは入力欄と同じ行から始まるうえ
/// 左端が右にずれるので、`┌` を左から探すと入力欄のほうが先に当たる。
/// 選択肢の文字そのものを目印にする。
#[test]
fn the_transfer_choice_is_highlighted_by_default() {
    let mut overlay = daw_overlay("I-IV");
    overlay.handle_key(
        KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE),
        Instant::now(),
    );

    let mut terminal = Terminal::new(TestBackend::new(80, 16)).unwrap();
    terminal.draw(|frame| draw(&overlay, frame)).unwrap();
    let buffer = terminal.backend().buffer();

    let transfer = row_with(buffer, '移');
    let keep = row_with(buffer, '確');
    assert_ne!(transfer, keep, "選択肢が同じ行に出ている");
    assert_ne!(buffer.cell(transfer).unwrap().bg, MONOKAI_BG);
    assert_eq!(buffer.cell(keep).unwrap().bg, MONOKAI_BG);
}

/// その文字が置かれている最初のセルの座標。
fn row_with(buffer: &ratatui::buffer::Buffer, needle: char) -> (u16, u16) {
    for y in 0..buffer.area.height {
        for x in 0..buffer.area.width {
            if buffer.cell((x, y)).unwrap().symbol() == needle.to_string() {
                return (x, y);
            }
        }
    }
    panic!("{needle} が見つからない");
}
