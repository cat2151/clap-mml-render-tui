//! Grid Sequencer の描画と mouse hit test が共有する pure layout。

use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::{LaneAddress, VisibleNoteRow, GRID_STEPS};

pub(super) const PATCH_WIDTH: usize = 24;
/// auto gain の表示幅。`-12.0`（下限）まで欠けずに入る。
pub(super) const GAIN_WIDTH: usize = 5;
pub(super) const LABEL_WIDTH: u16 = 42;
pub(super) const NOTE_CELL_WIDTH: u16 = 2;

const PATCH_START: u16 = 6;
/// GAIN 欄の左端。当たり判定は持たない（wheel も click も受けない表示専用の欄）ので、
/// テストが列を測るためだけに要る。
#[cfg(test)]
const GAIN_START: u16 = 31;
const NOTE_START: u16 = 37;
const NOTE_WIDTH: u16 = 4;

/// フレーズ型 list を出す右 pane の幅。`  UpDownHold ` と枠線が収まる最小。
pub(super) const PATTERN_LIST_WIDTH: u16 = 14;
/// grid の中身がちょうど収まる幅。NOTE grid も値 grid も同じ 68 桁 + 枠線。
///
/// 端末がこれより広くても grid は横へ伸ばさない。中身は 16 step ぶんで固定なので、
/// 伸ばしても枠が広がるだけで、右の pane との間に大きな空白ができる。
const GRID_WIDTH: u16 = LABEL_WIDTH + GRID_STEPS as u16 * NOTE_CELL_WIDTH + 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridSequencerLayout {
    pub chord_line: Option<Rect>,
    pub note: Rect,
    pub cc1: Rect,
    pub velocity: Rect,
    /// フレーズ型 list を出す右 pane。chord mode off か幅不足なら `None`。
    pub pattern_list: Option<Rect>,
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
        chord_on: bool,
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
        let rows = chunks[grid_index];
        // フレーズ型を送れるのは chord mode 中だけ。list も、grid を1 step も削らずに
        // 置ける幅があるときにしか出さない。
        let list_fits = chord_on && rows.width >= GRID_WIDTH + PATTERN_LIST_WIDTH;
        // grid と list をぴったり隣接させた塊を、画面の横中央へ置く。余りは左右へ
        // 均等に散らす。片側へ寄せると、広い端末で塊と余白が離れて視線が飛ぶ。
        let grid_width = GRID_WIDTH.min(rows.width);
        let block_width = grid_width + if list_fits { PATTERN_LIST_WIDTH } else { 0 };
        let left = rows.x + (rows.width - block_width) / 2;
        // 高さは中身ぶんへ詰める。grid と同じ高さへ伸ばすと、縦に広い端末では
        // list の下にただの空白が伸びるだけになる。
        let pattern_list = list_fits.then(|| Rect {
            x: left + grid_width,
            width: PATTERN_LIST_WIDTH,
            height: super::pattern_list::height_for(rows.height),
            ..rows
        });
        let grids = Rect {
            x: left,
            width: grid_width,
            ..rows
        };
        let note_height = requested_grid_height(note_rows).min(grids.height);
        let rest = grids.height.saturating_sub(note_height);
        let velocity_minimum = u16::from(velocity_rows > 0 && rest > 0);
        let cc1_height = requested_grid_height(cc1_rows).min(rest - velocity_minimum);
        let velocity_height = requested_grid_height(velocity_rows).min(rest - cc1_height);
        Self {
            chord_line: chord_visible.then(|| Rect {
                x: left,
                width: block_width,
                ..chunks[0]
            }),
            note: Rect {
                height: note_height,
                ..grids
            },
            cc1: value_grid_area(grids, note_height, cc1_height),
            velocity: value_grid_area(grids, note_height + cc1_height, velocity_height),
            pattern_list,
            // ステータスとキーバインドは塊へ寄せない。どちらも幅いっぱいでようやく
            // 収まる長さで、中央寄せのぶん詰めると末尾の情報から欠けていく。
            status: chunks[grid_index + 1],
            keybind: chunks[grid_index + 2],
        }
    }

    /// step セルの左端の列。[`Self::hit_test`] の逆写像。
    ///
    /// grid は中央寄せで左端が端末幅と塊の幅で動くため、テストは列を直書きせず
    /// ここから引く。ずれれば `hit_test` と往復しなくなって落ちる。
    #[cfg(test)]
    pub(crate) fn step_column(&self, step: usize) -> u16 {
        self.content_column(LABEL_WIDTH + step as u16 * NOTE_CELL_WIDTH)
    }

    /// NOTE 欄（音高）の列。
    #[cfg(test)]
    pub(crate) fn note_column(&self) -> u16 {
        self.content_column(NOTE_START)
    }

    /// PATCH 欄（音色名）の列。
    #[cfg(test)]
    pub(crate) fn patch_column(&self) -> u16 {
        self.content_column(PATCH_START)
    }

    /// GAIN 欄（auto gain）の列。
    #[cfg(test)]
    pub(crate) fn gain_column(&self) -> u16 {
        self.content_column(GAIN_START)
    }

    #[cfg(test)]
    fn content_column(&self, offset: u16) -> u16 {
        self.note.x + 1 + offset
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
