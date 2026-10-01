use crossterm::event::KeyCode;
use ratatui::{
    backend::{Backend, TestBackend},
    buffer::Buffer,
    layout::Position,
    Terminal,
};

use super::{draw, key, render, rows_in, screen_with_mml, HEIGHT, WIDTH};
use crate::GuitarArticulationScreen;

/// 絞り込みが無いときの項目の行数（KS 3 行 + 4 段）。枠の内側はこの下に絞り込みの欄を 1 行持つ。
const LIST_ROWS: u16 = 7;

/// 枠の内側の最下段（絞り込みの欄）の行。
fn overlay_rows(buffer: &Buffer) -> (u16, String) {
    let rows = rows_in(buffer, buffer.area);
    let top = rows
        .iter()
        .position(|row| row.contains("j/k:"))
        .unwrap_or_else(|| panic!("overlay が無い: {rows:#?}")) as u16;
    let bottom_inner = top + LIST_ROWS + 1;
    (bottom_inner, rows[usize::from(bottom_inner)].clone())
}

fn open_with_query(query: &str) -> GuitarArticulationScreen {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(key(KeyCode::Char('t')));
    screen.handle_key_event(key(KeyCode::Char('/')));
    for ch in query.chars() {
        screen.handle_key_event(key(KeyCode::Char(ch)));
    }
    screen
}

#[test]
fn without_a_filter_the_bottom_row_is_blank_below_the_release_lane() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(key(KeyCode::Char('t')));
    let buffer = render(&screen);
    let (bottom_y, bottom) = overlay_rows(&buffer);
    assert!(!bottom.contains(" /"), "{bottom}");
    assert!(
        bottom.trim_matches(|ch| ch == ' ' || ch == '│').is_empty(),
        "{bottom}"
    );
    let above = &rows_in(&buffer, buffer.area)[usize::from(bottom_y) - 1];
    assert!(above.contains("n:position rel"), "{above}");
}

#[test]
fn the_input_sits_on_the_bottom_row_inside_the_frame_with_the_cursor() {
    let screen = open_with_query("trill");
    let mut terminal = Terminal::new(TestBackend::new(WIDTH, HEIGHT)).unwrap();
    terminal.draw(|f| draw(&screen, f)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    let (bottom_y, bottom) = overlay_rows(&buffer);
    assert!(bottom.contains(" /trill"), "{bottom}");
    // 枠の内側（左右の縦線の間）に描く。
    assert!(bottom.trim_start().starts_with('│'), "{bottom}");
    assert!(bottom.trim_end().ends_with('│'), "{bottom}");
    let rows = rows_in(&buffer, buffer.area);
    // trill 4 個はどれも KS のピッチの行なので、1 行に横へ並ぶ。
    let list: Vec<&String> = rows[..usize::from(bottom_y)]
        .iter()
        .filter(|row| row.contains("trill"))
        .collect();
    assert_eq!(list.len(), 1, "{list:#?}");
    for label in [
        "w:trill half",
        "x:trill whole",
        "y:trill min3",
        "z:trill maj3",
    ] {
        assert!(list[0].contains(label), "{label}: {}", list[0]);
    }

    let cursor = terminal.backend_mut().get_cursor_position().unwrap();
    assert_eq!(cursor.y, bottom_y);
    let slash_x = bottom.chars().take_while(|&ch| ch != '/').count() as u16;
    assert_eq!(
        cursor,
        Position::new(slash_x + 1 + "trill".len() as u16, bottom_y)
    );
}

#[test]
fn a_committed_query_stays_on_the_bottom_row() {
    let mut screen = open_with_query("scrape");
    screen.handle_key_event(key(KeyCode::Enter));
    let buffer = render(&screen);
    let (bottom_y, bottom) = overlay_rows(&buffer);
    assert!(bottom.contains(" /scrape"), "{bottom}");
    let rows = rows_in(&buffer, buffer.area);
    let top = usize::from(bottom_y - LIST_ROWS);
    assert!(rows[top].contains("g:pick scratch"), "{}", rows[top]);
}
