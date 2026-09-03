//! Grid history を import した直後の mixer 初期値を、先頭 1 小節の cache から決め直す。
//!
//! Grid 画面で preview を聴いてから import した場合は、そのとき測った値が
//! [`crate::DawGridImportSong::track_volumes_db`] に載ってくるのでここは動かない。
//! preview を聴かずに import したときだけ、`kick_all_pending()` が走らせる
//! 先頭小節の cache render が終わるのを待って同じ計算をやり直す。

use std::sync::Arc;

use cmrt_tui_core::mixer::auto_trim::{auto_trim_volumes_db, measure_track_level, TrackLevel};

use super::{CacheState, DawApp, FIRST_PLAYABLE_TRACK};

/// mixer 初期値を測る小節（1 始まり）。
const AUTO_TRIM_MEASURE: usize = 1;

impl DawApp {
    /// 先頭小節の cache が出そろい次第 mixer 初期値を決めるよう予約する。
    pub(super) fn request_auto_trim_from_first_measure(&mut self) {
        self.pending_auto_trim = true;
    }

    /// 予約済みなら、先頭小節の cache から mixer 初期値を確定する。
    /// まだ render 中の track があるあいだは何もしない（メインループが毎 tick 呼ぶ）。
    pub(crate) fn pump_pending_auto_trim(&mut self) {
        if !self.pending_auto_trim {
            return;
        }
        let Some(track_samples) = self.settled_first_measure_samples() else {
            return;
        };
        self.pending_auto_trim = false;

        // 二乗和は cache ロックの外で回す（cache worker と競合させない）。
        let levels: Vec<TrackLevel> = track_samples
            .iter()
            .filter_map(|(track, samples)| measure_track_level(*track, samples))
            .collect();
        if levels.is_empty() {
            self.append_log_line("mixer: 先頭小節に測れる音が無いので初期値は 0dB のまま");
            return;
        }

        self.track_volumes_db = auto_trim_volumes_db(&levels, self.editor.tracks);
        self.save();
        self.append_log_line(format!(
            "mixer: 先頭小節から初期音量を決定 {}",
            self.auto_trim_log_summary()
        ));
    }

    /// 先頭小節の全 playable track が終着状態なら、測定に使えるサンプルを集めて返す。
    /// `Pending` / `Rendering` の track が残っているあいだは `None`。
    fn settled_first_measure_samples(&self) -> Option<Vec<(usize, Arc<Vec<f32>>)>> {
        let cache = self.cache.lock().unwrap();
        let mut track_samples = Vec::new();
        for track in FIRST_PLAYABLE_TRACK..self.editor.tracks {
            let cell = cache
                .get(track)
                .and_then(|row| row.get(AUTO_TRIM_MEASURE))?;
            match cell.state {
                CacheState::Pending | CacheState::Rendering => return None,
                // 空セルと render 失敗は待っても埋まらない。測れるぶんだけで決める。
                CacheState::Empty | CacheState::Error => continue,
                CacheState::Ready => {}
            }
            // サイズ上限を超えた cell は Ready でもサンプルを保持していない。
            if let Some(samples) = cell.samples.clone() {
                track_samples.push((track, samples));
            }
        }
        Some(track_samples)
    }

    fn auto_trim_log_summary(&self) -> String {
        (FIRST_PLAYABLE_TRACK..self.editor.tracks)
            .map(|track| {
                format!(
                    "track{}={}dB",
                    crate::tracks::track_display_number(track),
                    self.track_volume_db(track)
                )
            })
            .collect::<Vec<_>>()
            .join(" ")
    }
}

#[cfg(test)]
mod tests;
