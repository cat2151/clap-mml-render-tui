//! overlay preview（patch select / EFFECT CHAIN）が鳴らす縮小グリッドの組み立て。

use super::cell_has_content;
use crate::{DawApp, FIRST_PLAYABLE_TRACK};

/// overlay preview で対象セルが鳴らないときに使う、鳴らすためだけの短いフレーズ。
const PREVIEW_FALLBACK_PHRASE: &str = "c";

/// overlay preview の縮小グリッドで、対象セルが鳴らないなら [`PREVIEW_FALLBACK_PHRASE`] を
/// 書き込む。
///
/// 「鳴らない」の判定は [`cell_has_content`]（生の文字列が空かではない）。手書きが空でも
/// init に生成キーがあれば chord 行から生成されるので、そのセルを fallback で潰すと
/// 本番と違うフレーズが鳴る。init を差し替えてから呼ぶこと。
pub(crate) fn fill_preview_fallback_phrase(data: &mut [Vec<String>], track: usize, measure: usize) {
    if !cell_has_content(data, track, measure) {
        data[track][measure] = PREVIEW_FALLBACK_PHRASE.to_string();
    }
}

impl DawApp {
    /// overlay の preview 用に、conductor 行・chord 行・カーソル track だけを抜き出した
    /// 縮小グリッドを作る。
    ///
    /// **行の並びは本物のグリッドと同じにすること。** `build_cell_mml_from_data` は
    /// 行 index で行の役割（conductor / chord 行 / 演奏 track）を判断するので、
    /// 詰めて並べると別の役割の行として解釈される。
    pub(crate) fn preview_grid_for_cursor_track(&self) -> Vec<Vec<String>> {
        self.preview_grid_for_track(self.editor.cursor_track)
    }

    /// [`Self::preview_grid_for_cursor_track`] の、track を指定する形。
    pub(crate) fn preview_grid_for_track(&self, track: usize) -> Vec<Vec<String>> {
        let mut grid: Vec<Vec<String>> = (0..FIRST_PLAYABLE_TRACK)
            .map(|track| self.editor.data[track].clone())
            .collect();
        grid.push(self.editor.data[track].clone());
        grid
    }
}
