//! DAW セルキャッシュの管理

use std::path::PathBuf;

use cmrt_history::{daw_cache_mml_hash, DawCachedMeasure};

use super::mml::{cell_has_content, cell_is_generated_from_chord_row};
use super::tracks::track_renders_audio;
use super::{CacheState, CellCache, DawApp, WorkspaceKind, CHORD_TRACK, FIRST_PLAYABLE_TRACK};

fn workspace_cache_dir(root: &std::path::Path, workspace_kind: WorkspaceKind) -> PathBuf {
    match workspace_kind {
        WorkspaceKind::Persistent => root.to_path_buf(),
        WorkspaceKind::Daily => root.join("daily"),
    }
}

pub(super) fn ensure_workspace_cache_dir(workspace_kind: WorkspaceKind) -> anyhow::Result<PathBuf> {
    let root = cmrt_core::ensure_daw_cache_dir()?;
    let dir = workspace_cache_dir(&root, workspace_kind);
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn cache_wav_path(workspace_kind: WorkspaceKind, track: usize, measure: usize) -> Option<PathBuf> {
    if measure == 0 {
        return None;
    }
    ensure_workspace_cache_dir(workspace_kind)
        .ok()
        .map(|daw_dir| daw_dir.join(format!("track{}_meas{}.wav", track, measure)))
}

impl DawApp {
    // ─── キャッシュ管理 ───────────────────────────────────────

    /// data の内容に合わせてキャッシュ状態を同期する（data 変更後に呼ぶ）
    pub(super) fn sync_cache_states(&self) {
        let mut cache = self.cache.lock().unwrap();
        for t in 0..self.editor.tracks {
            for m in 0..=self.editor.measures {
                if m == 0 || !track_renders_audio(t) || !cell_has_content(&self.editor.data, t, m) {
                    cache[t][m] = CellCache::empty();
                } else if cache[t][m].state == CacheState::Empty {
                    cache[t][m].set_pending();
                }
            }
        }
    }

    /// 指定セルのキャッシュを無効化して状態を更新する
    pub(super) fn invalidate_cell(&self, track: usize, measure: usize) {
        if let Some(path) = cache_wav_path(self.workspace_kind, track, measure) {
            let _ = std::fs::remove_file(path);
        }
        let mut cache = self.cache.lock().unwrap();
        if measure == 0
            || !track_renders_audio(track)
            || !cell_has_content(&self.editor.data, track, measure)
        {
            cache[track][measure] = CellCache::empty();
        } else {
            cache[track][measure].set_pending();
        }
    }

    /// 指定セルのキャッシュジョブ内容をスナップショットとして構築する。
    ///
    /// セル自身の内容（`data[track][measure]`）が空のときはジョブを作らない。
    /// 以前は `build_cell_mml()` の結果（track0 を含む結合 MML）で空判定していたため、
    /// セルの内容を消去しても `●` インジケータが消えないバグがあった（issue #69 参照）。
    pub(super) fn prepare_cache_job(
        &self,
        track: usize,
        measure: usize,
    ) -> Option<super::CacheJob> {
        if measure == 0 {
            return None;
        }
        // chord 行の中身は MML ではなくコード進行なので、レンダリングにかけない。
        if !track_renders_audio(track) {
            return None;
        }
        // セル自身の内容が空なら投入しない（track0 含む結合 MML で判定しない）。
        // ただし chord 行から生成されるセルは、手書きが空でも中身がある。
        if !cell_has_content(&self.editor.data, track, measure) {
            return None;
        }
        let mml = self.build_cell_mml(track, measure);
        let rendered_mml_hash = daw_cache_mml_hash(&mml);
        let generation = self.cache.lock().unwrap()[track][measure].generation;
        let measure_samples = self.measure_duration_samples();
        Some(super::CacheJob {
            track,
            measure,
            measure_samples,
            generation,
            rendered_mml_hash,
            mml,
        })
    }

    pub(super) fn mark_cache_rendering(&self, track: usize, measure: usize) {
        Self::mark_cache_rendering_in(&self.cache, track, measure);
    }

    pub(super) fn mark_cache_rendering_in(
        cache: &std::sync::Arc<std::sync::Mutex<Vec<Vec<CellCache>>>>,
        track: usize,
        measure: usize,
    ) {
        let mut cache = cache.lock().unwrap();
        let Some(cell) = cache.get_mut(track).and_then(|row| row.get_mut(measure)) else {
            return;
        };
        cell.state = CacheState::Rendering;
        cell.rendered_mml_hash = None;
    }

    /// 指定セルのキャッシュジョブをワーカーキューに投入する
    pub(super) fn kick_cache(&self, track: usize, measure: usize) {
        // チャネルが既に閉じていれば送信は無視する（DawApp 終了後の残留呼び出しへの安全策）
        if let Some(job) = self.prepare_cache_job(track, measure) {
            self.mark_cache_rendering(track, measure);
            let _ = self.cache_tx.send(job);
        }
    }

    pub(super) fn invalidate_dependent_cells(
        &self,
        track: usize,
        measure: usize,
    ) -> Vec<(usize, usize)> {
        let mut affected = Vec::new();
        if track == 0 {
            // track0 セル変更: 全演奏トラックの全小節が影響を受ける
            let mut cache = self.cache.lock().unwrap();
            for t in FIRST_PLAYABLE_TRACK..self.editor.tracks {
                for m in 1..=self.editor.measures {
                    if !cell_has_content(&self.editor.data, t, m) {
                        cache[t][m] = CellCache::empty();
                    } else {
                        if let Some(path) = cache_wav_path(self.workspace_kind, t, m) {
                            let _ = std::fs::remove_file(path);
                        }
                        cache[t][m].set_pending();
                        affected.push((t, m));
                    }
                }
            }
        } else if track == CHORD_TRACK {
            // chord 行セル変更: そこから生成される演奏セルが影響を受ける。
            // init セル（measure 0）は全小節に効くので全小節、
            // measure セルはその小節だけ。
            let mut cache = self.cache.lock().unwrap();
            let measures: Vec<usize> = if measure == 0 {
                (1..=self.editor.measures).collect()
            } else {
                vec![measure]
            };
            for t in FIRST_PLAYABLE_TRACK..self.editor.tracks {
                for &m in &measures {
                    if !cell_is_generated_from_chord_row(&self.editor.data, t, m) {
                        continue;
                    }
                    if cell_has_content(&self.editor.data, t, m) {
                        if let Some(path) = cache_wav_path(self.workspace_kind, t, m) {
                            let _ = std::fs::remove_file(path);
                        }
                        cache[t][m].set_pending();
                        affected.push((t, m));
                    } else {
                        cache[t][m] = CellCache::empty();
                    }
                }
            }
        } else if measure == 0 && track_renders_audio(track) {
            // 音色セル（data[track][0]）変更: 同トラックの全小節が影響を受ける（issue #67 参照）
            let mut cache = self.cache.lock().unwrap();
            for m in 1..=self.editor.measures {
                if !cell_has_content(&self.editor.data, track, m) {
                    cache[track][m] = CellCache::empty();
                } else {
                    if let Some(path) = cache_wav_path(self.workspace_kind, track, m) {
                        let _ = std::fs::remove_file(path);
                    }
                    cache[track][m].set_pending();
                    affected.push((track, m));
                }
            }
        }
        affected
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
        for (track, measure) in self.invalidate_dependent_cells(track, measure) {
            self.kick_cache(track, measure);
        }
        // measure > 0 かつ track > 0 の場合は依存セルなし
    }

    /// Pending 状態のすべてのセルをワーカーキューに投入する
    pub(super) fn kick_all_pending(&self) {
        let pending: Vec<(usize, usize)> = {
            let cache = self.cache.lock().unwrap();
            (0..self.editor.tracks)
                .flat_map(|t| (0..=self.editor.measures).map(move |m| (t, m)))
                .filter(|&(t, m)| cache[t][m].state == CacheState::Pending)
                .collect()
        };
        for (t, m) in pending {
            self.kick_cache(t, m);
        }
    }

    pub(super) fn restore_cache_from_metadata(&self, cached_measures: &[DawCachedMeasure]) {
        let mut cache = self.cache.lock().unwrap();
        for t in 0..self.editor.tracks {
            for m in 1..=self.editor.measures {
                let Some(saved) = cached_measures
                    .iter()
                    .find(|entry| entry.track == t && entry.measure == m)
                else {
                    continue;
                };
                if !cell_has_content(&self.editor.data, t, m) {
                    continue;
                }
                let current_mml_hash = daw_cache_mml_hash(&self.build_cell_mml(t, m));
                if current_mml_hash != saved.mml_hash {
                    continue;
                }
                let Some(path) = cache_wav_path(self.workspace_kind, t, m) else {
                    continue;
                };
                match cmrt_tui_core::wav_io::read_wav_cache_info(&path) {
                    Ok(info)
                        if info.spec.sample_rate == self.cfg.sample_rate as u32
                            && info.spec.channels == 2 =>
                    {
                        cache[t][m].state = CacheState::Ready;
                        cache[t][m].rendered_mml_hash = Some(saved.mml_hash);
                        if info.interleaved_sample_count <= super::MAX_CACHED_SAMPLES {
                            match cmrt_tui_core::wav_io::load_wav_samples(&path) {
                                Ok(samples) => {
                                    cache[t][m].samples = Some(std::sync::Arc::new(samples));
                                    cache[t][m].rendered_measure_samples =
                                        Some(self.measure_duration_samples());
                                }
                                Err(_) => {
                                    cache[t][m].state = CacheState::Pending;
                                    cache[t][m].samples = None;
                                    cache[t][m].rendered_measure_samples = None;
                                    cache[t][m].rendered_mml_hash = None;
                                }
                            }
                        } else {
                            cache[t][m].samples = None;
                            cache[t][m].rendered_measure_samples = None;
                        }
                    }
                    Ok(_) | Err(_) => {
                        cache[t][m].state = CacheState::Pending;
                        cache[t][m].samples = None;
                        cache[t][m].rendered_measure_samples = None;
                        cache[t][m].rendered_mml_hash = None;
                    }
                }
            }
        }
    }

    pub(super) fn cached_measures_for_history(&self) -> Vec<DawCachedMeasure> {
        let cache = self.cache.lock().unwrap();
        let mut cached_measures = Vec::new();
        for t in 0..self.editor.tracks {
            for m in 1..=self.editor.measures {
                let current_mml_hash = daw_cache_mml_hash(&self.build_cell_mml(t, m));
                if cache[t][m].state == CacheState::Ready
                    && cache[t][m].rendered_mml_hash == Some(current_mml_hash)
                    && cell_has_content(&self.editor.data, t, m)
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

#[cfg(test)]
mod tests;
