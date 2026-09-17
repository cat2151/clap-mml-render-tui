//! グリッドの小節数を増減する。
//!
//! 増やすだけなら HTTP 経路（`http_server::app::ensure_http_grid_size`）が既にあるが、
//! こちらは**減らす**方も扱う。減らすときに範囲外になるものを 1 か所で片付ける:
//! 小節 index を持つ状態（カーソル・A-B repeat・`u` の記録・track ごとの
//! 再レンダリング batch）は、そのまま残すと次の参照で index out of bounds になる。
//!
//! 増やすときは、演奏 track の新しい小節に**最後の小節の内容を写す**。空で足すと
//! drum のような「全小節おなじ手書き」の track が、増えた小節だけ無音になる。
//! chord 行から生成している track は手書きが空なので、写しても空のまま chord 行に従う。

use super::{AbRepeatState, CellCache, DawApp, CHORD_TRACK, FIRST_PLAYABLE_TRACK};

impl DawApp {
    /// 小節数を `measures` にそろえる（init 列は数に含めない）。
    ///
    /// 減らしたときに切り捨てた、chord 行以外の非空セルの数を返す（呼び出し側が
    /// ログに出す用）。chord 行を数えないのは、呼び出し側が chord 行を書き直す前提だから。
    ///
    /// 増やしたときに演奏 track へ写した小節は、ここで render キューへ投入する。
    /// playback 側の vector（`measure_mmls` 等）はここでは触らない。呼び出し側が
    /// セルを書き終えたあとに `sync_playback_mml_state` で作り直す。
    pub(crate) fn set_measure_count(&mut self, measures: usize) -> usize {
        let previous_measures = self.editor.measures;
        if measures == previous_measures {
            return 0;
        }
        self.stop_play();

        let columns = measures + 1;
        let mut dropped = 0;
        for (track, row) in self.editor.data.iter_mut().enumerate() {
            let last_measure = row.get(previous_measures).cloned().unwrap_or_default();
            let fill = if track >= FIRST_PLAYABLE_TRACK && previous_measures > 0 {
                last_measure
            } else {
                String::new()
            };
            for (measure, cell) in row.iter_mut().enumerate().skip(columns) {
                if track != CHORD_TRACK && !cell.trim().is_empty() {
                    dropped += 1;
                }
                if let Some(path) =
                    super::cache::cache_wav_path(self.workspace_kind, track, measure)
                {
                    let _ = std::fs::remove_file(path);
                }
            }
            row.resize_with(columns, || fill.clone());
        }
        {
            let mut cache = self.cache.lock().unwrap();
            for row in cache.iter_mut() {
                row.resize_with(columns, CellCache::empty);
            }
        }
        self.editor.measures = measures;
        for track in FIRST_PLAYABLE_TRACK..self.editor.tracks {
            for measure in previous_measures + 1..columns {
                self.invalidate_cell(track, measure);
                self.kick_cache(track, measure);
            }
        }
        for batch in self.track_rerender_batches.lock().unwrap().iter_mut() {
            *batch = None;
        }
        *self.playback.ab_repeat.lock().unwrap() = AbRepeatState::Off;

        self.editor.cursor_measure = self.editor.cursor_measure.min(measures);
        self.editor.cell_undo = None;
        self.sync_http_grid_snapshot();
        dropped
    }
}

#[cfg(test)]
mod tests;
