//! DAW セルキャッシュの管理

use std::path::PathBuf;

use crate::history::{daw_cache_mml_hash, DawCachedMeasure, DawSessionState};

use super::{CacheState, CellCache, DawApp};

fn cache_wav_path(track: usize, measure: usize) -> Option<PathBuf> {
    if measure == 0 {
        return None;
    }
    cmrt_core::ensure_daw_dir()
        .ok()
        .map(|daw_dir| daw_dir.join(format!("track{}_meas{}.wav", track, measure)))
}

impl DawApp {
    // ─── キャッシュ管理 ───────────────────────────────────────

    /// data の内容に合わせてキャッシュ状態を同期する（data 変更後に呼ぶ）
    pub(super) fn sync_cache_states(&self) {
        let mut cache = self.cache.lock().unwrap();
        for t in 0..self.tracks {
            for m in 0..=self.measures {
                if m == 0 || self.data[t][m].trim().is_empty() {
                    cache[t][m] = CellCache::empty();
                } else if cache[t][m].state == CacheState::Empty {
                    cache[t][m].set_pending();
                }
            }
        }
    }

    /// 指定セルのキャッシュを無効化して状態を更新する
    pub(super) fn invalidate_cell(&self, track: usize, measure: usize) {
        if let Some(path) = cache_wav_path(track, measure) {
            let _ = std::fs::remove_file(path);
        }
        let mut cache = self.cache.lock().unwrap();
        if measure == 0 || self.data[track][measure].trim().is_empty() {
            cache[track][measure] = CellCache::empty();
        } else {
            cache[track][measure].set_pending();
        }
    }

    /// 指定セルのキャッシュジョブをワーカーキューに投入する
    ///
    /// セル自身の内容（`data[track][measure]`）が空のときはジョブを投入しない。
    /// 以前は `build_cell_mml()` の結果（track0 を含む結合 MML）で空判定していたため、
    /// セルの内容を消去しても `●` インジケータが消えないバグがあった（issue #69 参照）。
    pub(super) fn kick_cache(&self, track: usize, measure: usize) {
        if measure == 0 {
            return;
        }
        // セル自身の内容が空なら投入しない（track0 含む結合 MML で判定しない）
        if self.data[track][measure].trim().is_empty() {
            return;
        }
        let mml = self.build_cell_mml(track, measure);
        let rendered_mml_hash = daw_cache_mml_hash(&mml);
        let generation = {
            let mut cache = self.cache.lock().unwrap();
            cache[track][measure].state = CacheState::Rendering;
            cache[track][measure].samples = None;
            cache[track][measure].rendered_mml_hash = None;
            cache[track][measure].generation
        };
        // チャネルが既に閉じていれば送信は無視する（DawApp 終了後の残留呼び出しへの安全策）
        let _ = self.cache_tx.send(super::CacheJob {
            track,
            measure,
            generation,
            rendered_mml_hash,
            mml,
        });
    }

    /// 依存セルを一括で無効化してキャッシュジョブを投入する。
    ///
    /// `build_cell_mml(t, m)` はセル自身の内容に加え track0（グローバルヘッダ）と
    /// 音色セル `data[t][0]` を参照するため、それらが変化した際に依存セルも再レンダリングが必要。
    ///
    /// - track == 0（グローバルヘッダ変更）→ 全演奏トラック（1..tracks）の全小節を再キャッシュ
    /// - measure == 0 かつ track > 0（音色変更）→ 同トラックの全小節（1..=measures）を再キャッシュ
    /// - それ以外 → 追加の依存セルなし（呼び出し元が個別に処理済み）
    pub(super) fn invalidate_and_kick_dependent_cells(&self, track: usize, measure: usize) {
        if track == 0 {
            // track0 セル変更: 全演奏トラックの全小節が影響を受ける
            {
                let mut cache = self.cache.lock().unwrap();
                for t in 1..self.tracks {
                    for m in 1..=self.measures {
                        if self.data[t][m].trim().is_empty() {
                            cache[t][m] = CellCache::empty();
                        } else {
                            if let Some(path) = cache_wav_path(t, m) {
                                let _ = std::fs::remove_file(path);
                            }
                            cache[t][m].set_pending();
                        }
                    }
                }
            }
            for t in 1..self.tracks {
                for m in 1..=self.measures {
                    self.kick_cache(t, m);
                }
            }
        } else if measure == 0 {
            // 音色セル（data[track][0]）変更: 同トラックの全小節が影響を受ける（issue #67 参照）
            {
                let mut cache = self.cache.lock().unwrap();
                for m in 1..=self.measures {
                    if self.data[track][m].trim().is_empty() {
                        cache[track][m] = CellCache::empty();
                    } else {
                        if let Some(path) = cache_wav_path(track, m) {
                            let _ = std::fs::remove_file(path);
                        }
                        cache[track][m].set_pending();
                    }
                }
            }
            for m in 1..=self.measures {
                self.kick_cache(track, m);
            }
        }
        // measure > 0 かつ track > 0 の場合は依存セルなし
    }

    /// Pending 状態のすべてのセルをワーカーキューに投入する
    pub(super) fn kick_all_pending(&self) {
        let pending: Vec<(usize, usize)> = {
            let cache = self.cache.lock().unwrap();
            (0..self.tracks)
                .flat_map(|t| (0..=self.measures).map(move |m| (t, m)))
                .filter(|&(t, m)| cache[t][m].state == CacheState::Pending)
                .collect()
        };
        for (t, m) in pending {
            self.kick_cache(t, m);
        }
    }

    pub(super) fn restore_cache_from_history(&self, history: &DawSessionState) {
        let mut cache = self.cache.lock().unwrap();
        for t in 0..self.tracks {
            for m in 1..=self.measures {
                let Some(saved) = history
                    .cached_measures
                    .iter()
                    .find(|entry| entry.track == t && entry.measure == m)
                else {
                    continue;
                };
                if self.data[t][m].trim().is_empty() {
                    continue;
                }
                let current_mml_hash = daw_cache_mml_hash(&self.build_cell_mml(t, m));
                if current_mml_hash != saved.mml_hash {
                    continue;
                }
                let Some(path) = cache_wav_path(t, m) else {
                    continue;
                };
                match super::wav_io::read_wav_cache_info(&path) {
                    Ok(info)
                        if info.spec.sample_rate == self.cfg.sample_rate as u32
                            && info.spec.channels == 2 =>
                    {
                        cache[t][m].state = CacheState::Ready;
                        cache[t][m].rendered_mml_hash = Some(saved.mml_hash);
                        if info.interleaved_sample_count <= super::MAX_CACHED_SAMPLES {
                            match super::wav_io::load_wav_samples(&path) {
                                Ok(samples) => {
                                    cache[t][m].samples = Some(std::sync::Arc::new(samples));
                                }
                                Err(_) => {
                                    cache[t][m].state = CacheState::Pending;
                                    cache[t][m].samples = None;
                                    cache[t][m].rendered_mml_hash = None;
                                }
                            }
                        } else {
                            cache[t][m].samples = None;
                        }
                    }
                    Ok(_) | Err(_) => {
                        cache[t][m].state = CacheState::Pending;
                        cache[t][m].samples = None;
                        cache[t][m].rendered_mml_hash = None;
                    }
                }
            }
        }
    }

    pub(super) fn cached_measures_for_history(&self) -> Vec<DawCachedMeasure> {
        let cache = self.cache.lock().unwrap();
        let mut cached_measures = Vec::new();
        for t in 0..self.tracks {
            for m in 1..=self.measures {
                let current_mml_hash = daw_cache_mml_hash(&self.build_cell_mml(t, m));
                if cache[t][m].state == CacheState::Ready
                    && cache[t][m].rendered_mml_hash == Some(current_mml_hash)
                    && !self.data[t][m].trim().is_empty()
                {
                    cached_measures.push(DawCachedMeasure {
                        track: t,
                        measure: m,
                        mml_hash: current_mml_hash,
                        legacy_mml: None,
                    });
                }
            }
        }
        cached_measures
    }
}
