//! Grid Sequencer の描画と mouse hit test が共有する pure layout。

use ratatui::layout::{Constraint, Direction, Layout, Rect};

use crate::{LaneAddress, VisibleNoteRow, GRID_STEPS};

pub(super) const PATCH_WIDTH: usize = 24;
/// auto gain の表示幅。`-12.0`（下限）まで欠けずに入る。
pub(super) const GAIN_WIDTH: usize = 5;
/// swing の表示幅。`50`〜`66` と非適用の `-`、見出しの `SW` がちょうど入る。
pub(super) const SWING_WIDTH: usize = 2;
pub(super) const LABEL_WIDTH: u16 = 45;
pub(super) const NOTE_CELL_WIDTH: u16 = 2;

const PATCH_START: u16 = 6;
/// GAIN 欄の左端。当たり判定は持たない（wheel も click も受けない表示専用の欄）ので、
/// テストが列を測るためだけに要る。
#[cfg(test)]
const GAIN_START: u16 = 31;
const NOTE_START: u16 = 37;
const NOTE_WIDTH: u16 = 4;
/// SWING 欄の左端。GAIN と同じく表示専用なので当たり判定は持たない。
#[cfg(test)]
const SWING_START: u16 = 42;

/// フレーズ型listを2列（arp+bass / drum）で出す右paneの幅。
/// 1列12桁×2 + 左右の枠線。
pub(super) const PATTERN_LIST_WIDTH: u16 = 26;
/// 1列へ畳んでも最長labelを欠かさない従来幅。
pub(super) const PATTERN_LIST_MIN_WIDTH: u16 = 14;
/// grid の中身がちょうど収まる幅。NOTE grid も値 grid も同じ 68 桁 + 枠線。
///
/// 端末がこれより広くても grid は横へ伸ばさない。中身は 16 step ぶんで固定なので、
/// 伸ばしても枠が広がるだけで、右の pane との間に大きな空白ができる。
pub(super) const GRID_WIDTH: u16 = LABEL_WIDTH + GRID_STEPS as u16 * NOTE_CELL_WIDTH + 2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GridSequencerLayout {
    pub chord_line: Option<Rect>,
    pub note: Rect,
    /// NOTE の最終 track の直下に置く、auto random patch のロード進捗欄。
    pub patch_load_progress: Rect,
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
    /// `pattern_list_sections` は右 pane に出す section それぞれの行数（優先順）。
    /// 空なら pane を出さない。中身は [`super::pattern_list::section_heights`] が決める。
    pub fn new(
        area: Rect,
        note_rows: usize,
        cc1_rows: usize,
        velocity_rows: usize,
        chord_visible: bool,
        pattern_list_sections: &[usize],
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
        // list は、grid を1 step も削らずに置ける幅があるときにしか出さない。
        let two_columns = rows.width >= GRID_WIDTH + PATTERN_LIST_WIDTH;
        let list_width = if two_columns {
            PATTERN_LIST_WIDTH
        } else {
            PATTERN_LIST_MIN_WIDTH
        };
        let narrow_height = [pattern_list_sections.iter().sum::<usize>()];
        let heights = if two_columns {
            pattern_list_sections
        } else {
            &narrow_height
        };
        let list_height = super::pattern_list::height_for(heights, rows.height);
        let list_fits = list_height > 0 && rows.width >= GRID_WIDTH + PATTERN_LIST_MIN_WIDTH;
        // grid と list をぴったり隣接させた塊を、画面の横中央へ置く。余りは左右へ
        // 均等に散らす。片側へ寄せると、広い端末で塊と余白が離れて視線が飛ぶ。
        let grid_width = GRID_WIDTH.min(rows.width);
        let block_width = grid_width + if list_fits { list_width } else { 0 };
        let left = rows.x + (rows.width - block_width) / 2;
        // 高さは中身ぶんへ詰める。grid と同じ高さへ伸ばすと、縦に広い端末では
        // list の下にただの空白が伸びるだけになる。
        let pattern_list = list_fits.then(|| Rect {
            x: left + grid_width,
            width: list_width,
            height: list_height,
            ..rows
        });
        let grids = Rect {
            x: left,
            width: grid_width,
            ..rows
        };
        let note_height = requested_grid_height(note_rows).min(grids.height);
        let progress_height = u16::from(grids.height > note_height);
        let rest = grids
            .height
            .saturating_sub(note_height)
            .saturating_sub(progress_height);
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
            patch_load_progress: value_grid_area(grids, note_height, progress_height),
            cc1: value_grid_area(grids, note_height + progress_height, cc1_height),
            velocity: value_grid_area(
                grids,
                note_height + progress_height + cc1_height,
                velocity_height,
            ),
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

    /// SWING 欄（shuffle 量）の列。
    #[cfg(test)]
    pub(crate) fn swing_column(&self) -> u16 {
        self.content_column(SWING_START)
    }

    #[cfg(test)]
    fn content_column(&self, offset: u16) -> u16 {
        self.note.x + 1 + offset
    }

    /// `address` の lane が描かれる端末行。[`Self::hit_test`] の逆写像。
    ///
    /// 行の並びは chord mode と drum 行の有無で動く。列と同じく、テストは行も
    /// 直書きせずここから引く。
    #[cfg(test)]
    pub(crate) fn lane_line(&self, visible_rows: &[VisibleNoteRow], address: LaneAddress) -> u16 {
        let index = visible_rows
            .iter()
            .position(|row| row.address == address)
            .expect("the lane is visible");
        self.note.y + 2 + u16::try_from(index).expect("visible rows fit in u16")
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
