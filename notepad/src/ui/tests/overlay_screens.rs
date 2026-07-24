use super::*;
use crate::{PatchPhrasePane, PatchSelectPane};

fn pane_contains_cursor_highlight(buffer: &Buffer, pane: ratatui::layout::Rect) -> bool {
    (pane.y..pane.y + pane.height).any(|y| {
        (pane.x..pane.x + pane.width).any(|x| {
            let cell = buffer.cell((x, y)).unwrap();
            cell.bg == cursor_highlight_bg(cell.fg)
        })
    })
}

mod notepad_history;
mod patch_phrase;
mod patch_select;
