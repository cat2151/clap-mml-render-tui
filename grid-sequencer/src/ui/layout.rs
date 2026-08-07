//! Grid Sequencer の描画と mouse hit test が共有する pure layout。

use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::{LaneAddress, VisibleNoteRow, GRID_STEPS};

pub(super) const PATCH_WIDTH: usize = 24;
pub(super) const LABEL_WIDTH: u16 = 36;
pub(super) const NOTE_CELL_WIDTH: u16 = 2;

const PATCH_START: u16 = 6;
const NOTE_START: u16 = 31;
const NOTE_WIDTH: u16 = 4;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridSequencerLayout {
    pub chord_line: Option<Rect>,
    pub note: Rect,
    pub cc1: Rect,
    pub velocity: Rect,
    pub status: Rect,
    pub keybind: Rect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GridHit {
    NoteCell { address: LaneAddress, step: usize },
    LaneNote { address: LaneAddress },
    InstancePatch { instance: usize },
}

impl GridSequencerLayout {
    pub fn new(
        area: Rect,
        note_rows: usize,
        cc1_rows: usize,
        velocity_rows: usize,
        chord_visible: bool,
    ) -> Self {
        let mut constraints = Vec::with_capacity(4);
        if chord_visible {
            constraints.push(Constraint::Length(1));
        }
        constraints.extend([
            Constraint::Min(5),
            Constraint::Length(1),
            Constraint::Length(1),
        ]);
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(area);
        let grid_index = usize::from(chord_visible);
        let grids = chunks[grid_index];
        let note_height = requested_grid_height(note_rows).min(grids.height);
        let rest = grids.height.saturating_sub(note_height);
        let velocity_minimum = u16::from(velocity_rows > 0 && rest > 0);
        let cc1_height = requested_grid_height(cc1_rows).min(rest - velocity_minimum);
        let velocity_height = requested_grid_height(velocity_rows).min(rest - cc1_height);
        Self {
            chord_line: chord_visible.then_some(chunks[0]),
            note: Rect {
                height: note_height,
                ..grids
            },
            cc1: value_grid_area(grids, note_height, cc1_height),
            velocity: value_grid_area(grids, note_height + cc1_height, velocity_height),
            status: chunks[grid_index + 1],
            keybind: chunks[grid_index + 2],
        }
    }

    pub fn hit_test(
        &self,
        column: u16,
        line: u16,
        visible_rows: &[VisibleNoteRow],
    ) -> Option<GridHit> {
        let content_left = self.note.x.checked_add(1)?;
        let content_right = self
            .note
            .x
            .saturating_add(self.note.width)
            .saturating_sub(1);
        let first_row = self.note.y.checked_add(2)?;
        let rows_bottom = self
            .note
            .y
            .saturating_add(self.note.height)
            .saturating_sub(1);
        if column < content_left
            || column >= content_right
            || line < first_row
            || line >= rows_bottom
        {
            return None;
        }
        let flat_row = usize::from(line - first_row);
        let row = *visible_rows.get(flat_row)?;
        let x = column - content_left;
        if (PATCH_START..PATCH_START + PATCH_WIDTH as u16).contains(&x) {
            return Some(GridHit::InstancePatch {
                instance: row.address.instance,
            });
        }
        if (NOTE_START..NOTE_START + NOTE_WIDTH).contains(&x) {
            return Some(GridHit::LaneNote {
                address: row.address,
            });
        }
        let cell_x = x.checked_sub(LABEL_WIDTH)?;
        let step = usize::from(cell_x / NOTE_CELL_WIDTH);
        (step < GRID_STEPS).then_some(GridHit::NoteCell {
            address: row.address,
            step,
        })
    }
}

fn requested_grid_height(rows: usize) -> u16 {
    u16::try_from(rows + 3).unwrap_or(u16::MAX)
}

fn value_grid_area(area: Rect, top: u16, height: u16) -> Rect {
    Rect {
        y: area.y.saturating_add(top),
        height,
        ..area
    }
}

#[cfg(test)]
mod tests;
