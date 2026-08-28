//! chord 行から生成される track が grid にどう見えるか。
//!
//! 生成対象セルは**手書きが空のまま音が鳴る**ので、空セルのままだと
//! 「ここは鳴る」ことも「何が鳴る」ことも画面から読めない。

use super::super::*;
use super::grid::{header_row, inject_catalog, row_symbols, x_of_in_row, TEST_BASS_PATCH};

// ─── chord 行から生成される track の見え方 ───────────────────

const TEST_CHORD_DIRECTIVE: &str = "close";

/// 音色と「chord 行から生成する」指定の両方が入った init セル。
fn generated_init_cell(display: &str, directive: &str) -> String {
    format!(r#"{{"Surge XT patch": "{display}", "generate from chord track": "{directive}"}}"#)
}

/// chord 行に `I-IV` があり、T1 だけがそこから生成される app。
fn app_with_a_generated_track() -> DawApp {
    let mut app = build_test_app();
    app.editor.data[crate::CHORD_TRACK][0] = "key:G".to_string();
    app.editor.data[crate::CHORD_TRACK][1] = "I-IV".to_string();
    app.editor.data[crate::FIRST_PLAYABLE_TRACK][0] =
        generated_init_cell(TEST_BASS_PATCH, TEST_CHORD_DIRECTIVE);
    inject_catalog(&mut app, &[TEST_BASS_PATCH]);
    app
}

#[test]
fn the_init_column_marks_a_track_that_is_generated_from_the_chord_row() {
    let app = app_with_a_generated_track();

    let buffer = render_buffer(&app, 60, 24);
    let header_y = header_row(&buffer);
    let track1_y = header_y + 1 + 2 * crate::FIRST_PLAYABLE_TRACK as u16;
    let init_x = x_of_in_row(&buffer, header_y, "Init");

    // セル本体は `*` 付きの音色名。`*` が音色名より先に来る（切り詰めても残る）。
    assert_eq!(x_of_in_row(&buffer, track1_y, "*bass:Wobble"), init_x);
    // 指定はセルの 1 行下（もともと空いている init 列のインジケータ行）。
    assert_eq!(
        x_of_in_row(&buffer, track1_y + 1, TEST_CHORD_DIRECTIVE),
        init_x
    );
    // 紫 = 手書きではなく chord 行に由来する表示。
    let mark = buffer.cell((init_x, track1_y)).unwrap();
    assert_eq!(mark.fg, MONOKAI_PURPLE);
}

#[test]
fn a_generated_cell_shows_the_chord_row_text_even_though_the_cell_is_empty() {
    let app = app_with_a_generated_track();
    assert!(app.editor.data[crate::FIRST_PLAYABLE_TRACK][1].is_empty());

    let buffer = render_buffer(&app, 60, 24);
    let header_y = header_row(&buffer);
    let track1_y = header_y + 1 + 2 * crate::FIRST_PLAYABLE_TRACK as u16;

    // chord 行と同じ文字が、同じ M1 列に、紫で出る。
    let m1_x = x_of_in_row(&buffer, header_y, "M1");
    assert_eq!(x_of_in_row(&buffer, track1_y, "I-IV"), m1_x);
    assert_eq!(buffer.cell((m1_x, track1_y)).unwrap().fg, MONOKAI_PURPLE);
}

#[test]
fn a_handwritten_cell_keeps_its_own_text_and_color() {
    let mut app = app_with_a_generated_track();
    // 生成対象 track でも、手で書いた小節は手書きが勝つ（4.5）。
    app.editor.data[crate::FIRST_PLAYABLE_TRACK][1] = "cdef".to_string();

    let buffer = render_buffer(&app, 60, 24);
    let header_y = header_row(&buffer);
    let track1_y = header_y + 1 + 2 * crate::FIRST_PLAYABLE_TRACK as u16;

    let m1_x = x_of_in_row(&buffer, header_y, "M1");
    assert_eq!(x_of_in_row(&buffer, track1_y, "cdef"), m1_x);
    assert_ne!(buffer.cell((m1_x, track1_y)).unwrap().fg, MONOKAI_PURPLE);
}

#[test]
fn a_track_without_the_generate_key_shows_nothing_from_the_chord_row() {
    let app = app_with_a_generated_track();

    let buffer = render_buffer(&app, 60, 24);
    let header_y = header_row(&buffer);
    let track2_y = header_y + 1 + 2 * (crate::FIRST_PLAYABLE_TRACK as u16 + 1);

    let row = row_symbols(&buffer, track2_y).concat();
    assert!(!row.contains("I-IV"), "row: {row:?}");
    assert!(!row.contains('*'), "row: {row:?}");
}

/// chord 行の init セルは chord2mml への指定（`key:G`）をそのまま出す。
#[test]
fn the_chord_row_init_column_shows_the_key() {
    let app = app_with_a_generated_track();

    let buffer = render_buffer(&app, 60, 24);
    let header_y = header_row(&buffer);
    let chord_y = header_y + 1 + 2 * crate::CHORD_TRACK as u16;
    let init_x = x_of_in_row(&buffer, header_y, "Init");

    assert_eq!(x_of_in_row(&buffer, chord_y, "key:G"), init_x);
}

// ─── `C` キーで画面上のカーソルが chord 行へ動くか ───────────────

/// その track ラベルが「カーソル行」として強調されているか。
///
/// grid は中央寄せされるので、ラベルの x は 0 ではない。呼び出し側が
/// 実際に描かれたラベルの x を渡すこと。
fn label_is_highlighted(buffer: &Buffer, label_x: u16, y: u16) -> bool {
    let cell = buffer.cell((label_x, y)).unwrap();
    cell.bg == cursor_highlight_bg(cell.fg)
}

fn label_row_y(buffer: &Buffer, track: usize) -> u16 {
    header_row(buffer) + 1 + 2 * track as u16
}

/// カーソル位置の state ではなく、**実際に描かれた強調**が動くことを見る。
#[test]
fn pressing_c_moves_the_drawn_cursor_to_the_chord_row_and_back() {
    let mut app = app_with_a_generated_track();
    app.editor.cursor_track = crate::FIRST_PLAYABLE_TRACK;
    app.editor.cursor_measure = 1;

    let buffer = render_buffer(&app, 60, 24);
    let chord_y = label_row_y(&buffer, crate::CHORD_TRACK);
    let track1_y = label_row_y(&buffer, crate::FIRST_PLAYABLE_TRACK);
    let label_x = x_of_in_row(&buffer, chord_y, "Chord");
    assert_eq!(x_of_in_row(&buffer, track1_y, "T1"), label_x);
    assert!(label_is_highlighted(&buffer, label_x, track1_y));
    assert!(!label_is_highlighted(&buffer, label_x, chord_y));

    app.handle_normal_key_event(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('C'),
        crossterm::event::KeyModifiers::SHIFT,
    ));

    let buffer = render_buffer(&app, 60, 24);
    assert!(label_is_highlighted(&buffer, label_x, chord_y));
    assert!(!label_is_highlighted(&buffer, label_x, track1_y));

    app.handle_normal_key_event(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('C'),
        crossterm::event::KeyModifiers::SHIFT,
    ));

    let buffer = render_buffer(&app, 60, 24);
    assert!(label_is_highlighted(&buffer, label_x, track1_y));
    assert!(!label_is_highlighted(&buffer, label_x, chord_y));
}
