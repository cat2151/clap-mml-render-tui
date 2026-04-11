use super::*;
use crate::tui::{PatchPhrasePane, PatchSelectPane};

fn pane_contains_cursor_highlight(buffer: &Buffer, pane: ratatui::layout::Rect) -> bool {
    (pane.y..pane.y + pane.height).any(|y| {
        (pane.x..pane.x + pane.width).any(|x| {
            let cell = buffer.cell((x, y)).unwrap();
            cell.bg == cursor_highlight_bg(cell.fg)
        })
    })
}

#[path = "overlay_screens/notepad_history.rs"]
mod notepad_history;
#[path = "overlay_screens/patch_phrase.rs"]
mod patch_phrase;
#[path = "overlay_screens/patch_select.rs"]
mod patch_select;
