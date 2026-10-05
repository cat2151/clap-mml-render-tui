//! kit の行一覧と、note number / step に結び付いた入力を保持する。

use std::ops::Range;

/// 4/4 の 1 小節を 16 分音符で編集する。terminal サイズでは変更しない。
pub const DRUM_STEPS: usize = 16;
const MIDI_NOTES: usize = 128;

#[derive(Clone, Debug)]
struct SelectedKit {
    name: String,
    /// None は旧 catalog / 抽出失敗、Some([]) は取得済みの空一覧。
    notes: Option<Vec<u8>>,
}

/// host が画面の往復中も所有し続ける状態。保存・復元の対象にはしない。
#[derive(Clone, Debug)]
pub struct DrumSequencerScreen {
    kit: Option<SelectedKit>,
    cells: [[bool; DRUM_STEPS]; MIDI_NOTES],
    pub(crate) cursor_row: usize,
    pub(crate) cursor_step: usize,
    scroll_top: usize,
}

impl Default for DrumSequencerScreen {
    fn default() -> Self {
        Self {
            kit: None,
            cells: [[false; DRUM_STEPS]; MIDI_NOTES],
            cursor_row: 0,
            cursor_step: 0,
            scroll_top: 0,
        }
    }
}

impl DrumSequencerScreen {
    /// catalog の一覧を採用する。非表示になった note の入力も残す。
    /// カーソルの note が残れば維持し、消えたら先頭へ移る。step は維持する。
    pub fn set_kit(&mut self, name: String, notes: Option<Vec<u8>>) {
        let previous_note = self.cursor_note();
        let notes = notes.map(|mut notes| {
            notes.retain(|note| usize::from(*note) < MIDI_NOTES);
            notes.sort_unstable();
            notes.dedup();
            notes
        });
        self.kit = Some(SelectedKit { name, notes });
        self.cursor_row = previous_note
            .and_then(|note| self.notes().binary_search(&note).ok())
            .unwrap_or(0);
        if self.cursor_row == 0 {
            self.scroll_top = 0;
        }
    }

    pub fn kit_name(&self) -> Option<&str> {
        self.kit.as_ref().map(|kit| kit.name.as_str())
    }

    /// 現在の kit の昇順・重複なしの行一覧。
    pub fn notes(&self) -> &[u8] {
        self.kit
            .as_ref()
            .and_then(|kit| kit.notes.as_deref())
            .unwrap_or(&[])
    }

    pub fn notes_known(&self) -> bool {
        self.kit.as_ref().is_some_and(|kit| kit.notes.is_some())
    }

    pub fn cursor_note(&self) -> Option<u8> {
        self.notes().get(self.cursor_row).copied()
    }

    /// 0..15 の内部 step index。
    pub fn cursor_step(&self) -> usize {
        self.cursor_step
    }

    pub fn cell_on(&self, note: u8, step: usize) -> bool {
        self.cells
            .get(usize::from(note))
            .and_then(|row| row.get(step))
            .copied()
            .unwrap_or(false)
    }

    pub(crate) fn toggle_cursor(&mut self) {
        if let Some(note) = self.cursor_note() {
            let cell = &mut self.cells[usize::from(note)][self.cursor_step];
            *cell = !*cell;
        }
    }

    /// 描画の高さが変わってもカーソルを見失わず、同じ viewport 内では行を動かさない。
    pub(crate) fn visible_rows(&mut self, height: usize) -> Range<usize> {
        if height == 0 || self.notes().is_empty() {
            return 0..0;
        }
        let max_top = self.notes().len().saturating_sub(height);
        self.scroll_top = self.scroll_top.min(max_top);
        if self.cursor_row < self.scroll_top {
            self.scroll_top = self.cursor_row;
        } else if self.cursor_row >= self.scroll_top + height {
            self.scroll_top = self.cursor_row + 1 - height;
        }
        self.scroll_top..(self.scroll_top + height).min(self.notes().len())
    }
}

#[cfg(test)]
mod tests;
