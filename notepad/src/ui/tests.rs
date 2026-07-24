use ratatui::layout::{Constraint, Direction, Layout, Position};
use ratatui::{backend::TestBackend, buffer::Buffer, style::Color, Terminal};

use cmrt_history::PatchPhraseState;
use cmrt_runtime::Config;
use cmrt_tui_core::theme::{
    cursor_highlight_bg, MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY, MONOKAI_GREEN,
    MONOKAI_PURPLE, MONOKAI_YELLOW,
};

use crate::NotepadScreen;

use super::{draw, status_color, Mode, PlayState};

mod cache_indicators;
mod colors;
mod cursor_style;
mod footer;
mod help_screens;
mod insert_screen;
mod overlay_screens;
mod sound_check_guide;

fn test_config() -> Config {
    crate::tests::test_config()
}

fn render_lines(app: &mut NotepadScreen<'static>, width: u16, height: u16) -> Vec<String> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(app, f)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

fn render_buffer(app: &mut NotepadScreen<'static>, width: u16, height: u16) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(app, f)).unwrap();
    terminal.backend().buffer().clone()
}

fn render_cursor_position(app: &mut NotepadScreen<'static>, width: u16, height: u16) -> Position {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(app, f)).unwrap();
    terminal.get_cursor_position().unwrap()
}

/// 空白を無視して文字列を探し、その先頭セルの座標を返す。
/// 罫線やパディングで空白が入るヘルプ・オーバーレイの検証に使う。
fn find_text_ignoring_spaces(buffer: &Buffer, text: &str) -> (u16, u16) {
    for y in 0..buffer.area.height {
        let mut normalized = String::new();
        let mut x_positions = Vec::new();
        for x in 0..buffer.area.width {
            let symbol = buffer
                .cell((x, y))
                .unwrap_or_else(|| panic!("failed to access buffer cell at ({x}, {y})"))
                .symbol();
            if symbol == " " || symbol.is_empty() {
                continue;
            }
            for ch in symbol.chars() {
                normalized.push(ch);
                x_positions.push(x);
            }
        }
        if let Some(byte_index) = normalized.find(text) {
            let char_index = normalized[..byte_index].chars().count();
            return (x_positions[char_index], y);
        }
    }
    panic!("text not found in buffer when ignoring spaces: {text}");
}

/// ヘルプ overlay の枠（左上・右下）の座標を返す。
fn help_overlay_bounds(buffer: &Buffer) -> (u16, u16, u16, u16) {
    let (title_x, top) = find_text_ignoring_spaces(buffer, "ヘルプ(Keybinds)");

    let mut left = title_x;
    while left > 0
        && buffer
            .cell((left, top))
            .unwrap_or_else(|| panic!("failed to access buffer cell at ({left}, {top})"))
            .symbol()
            != "┌"
    {
        left -= 1;
    }

    let mut right = title_x;
    while right + 1 < buffer.area.width
        && buffer
            .cell((right, top))
            .unwrap_or_else(|| panic!("failed to access buffer cell at ({right}, {top})"))
            .symbol()
            != "┐"
    {
        right += 1;
    }

    let mut bottom = top;
    while bottom + 1 < buffer.area.height {
        if buffer
            .cell((left, bottom))
            .unwrap_or_else(|| panic!("failed to access buffer cell at ({left}, {bottom})"))
            .symbol()
            == "└"
            && buffer
                .cell((right, bottom))
                .unwrap_or_else(|| panic!("failed to access buffer cell at ({right}, {bottom})"))
                .symbol()
                == "┘"
        {
            break;
        }
        bottom += 1;
    }

    (left, top, right, bottom)
}

fn find_text(buffer: &Buffer, text: &str) -> (u16, u16) {
    for y in 0..buffer.area.height {
        let line: String = (0..buffer.area.width)
            .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
            .collect();
        if let Some(x) = line.find(text) {
            return (x as u16, y);
        }
    }
    panic!("text not found in buffer: {text}");
}
