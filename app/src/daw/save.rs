//! DAW セッションの保存・読み込み

use super::{DawApp, FIRST_PLAYABLE_TRACK, MIXER_MAX_DB, MIXER_MIN_DB};

// ─── 保存形式 ─────────────────────────────────────────────────

/// DAW セッションの JSON 保存形式のルート。
#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct DawSaveFile {
    pub(super) tracks: Vec<DawSaveTrack>,
}

/// JSON 保存形式のトラックエントリ。空トラックは含まれない。
#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct DawSaveTrack {
    pub(super) track: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) volume_db: Option<i32>,
    pub(super) meas: Vec<DawSaveMeas>,
}

/// JSON 保存形式の小節エントリ。空小節は含まれない。
#[derive(serde::Serialize, serde::Deserialize)]
pub(super) struct DawSaveMeas {
    pub(super) meas: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(super) description: Option<String>,
    pub(super) mml: String,
}

/// 保存済みセッションを欠落なく読み込める `DawApp` の最小サイズを返す。
///
/// 返り値の 2 要素目は「列数」ではなく `DawApp.measures` に入れる値
/// （= 最大の playable measure index）である。
/// そのため data/cell/cache の確保では常に `measures + 1` 列を用いる。
pub(super) fn required_grid_size(file: &DawSaveFile) -> (usize, usize) {
    let mut tracks = FIRST_PLAYABLE_TRACK + 1;
    let mut measures = 1;
    for save_track in &file.tracks {
        tracks = tracks.max(save_track.track.saturating_add(1));
        for save_meas in &save_track.meas {
            // `save_meas.meas` 自体が `DawApp.measures` と同じ意味の index 値。
            // 例: meas=25 を保持するには 0..=25 を扱える `measures=25` が必要。
            measures = measures.max(save_meas.meas.max(1));
        }
    }
    (tracks, measures)
}

pub(super) fn load_saved_grid_size() -> Option<(usize, usize)> {
    let path = crate::history::daw_file_load_path()?;
    let content = std::fs::read_to_string(path).ok()?;
    let file = serde_json::from_str::<DawSaveFile>(&content).ok()?;
    Some(required_grid_size(&file))
}

/// data グリッドを `DawSaveFile` に変換する（空トラック・空小節は除外）。
pub(super) fn data_to_save_file(
    data: &[Vec<String>],
    track_volumes_db: &[i32],
    tracks: usize,
    measures: usize,
) -> DawSaveFile {
    let mut save_tracks: Vec<DawSaveTrack> = Vec::new();
    for (t, row) in data.iter().enumerate().take(tracks) {
        let mut save_meas: Vec<DawSaveMeas> = Vec::new();
        for (m, cell) in row.iter().enumerate().take(measures + 1) {
            if !cell.trim().is_empty() {
                let description = if m == 0 {
                    Some("initial".to_string())
                } else {
                    None
                };
                save_meas.push(DawSaveMeas {
                    meas: m,
                    description,
                    mml: cell.clone(),
                });
            }
        }
        let volume_db = track_volumes_db
            .get(t)
            .copied()
            .filter(|volume_db| *volume_db != 0);
        if !save_meas.is_empty() || volume_db.is_some() {
            let description = if t == 0 {
                Some("tempo track".to_string())
            } else {
                None
            };
            save_tracks.push(DawSaveTrack {
                track: t,
                description,
                volume_db,
                meas: save_meas,
            });
        }
    }
    DawSaveFile {
        tracks: save_tracks,
    }
}

/// `DawSaveFile` を data グリッドに書き込む（範囲外インデックスは無視）。
pub(super) fn apply_save_file_to_data(
    file: &DawSaveFile,
    data: &mut [Vec<String>],
    tracks: usize,
    measures: usize,
) {
    for save_track in &file.tracks {
        let t = save_track.track;
        if t >= tracks {
            continue;
        }
        for save_meas in &save_track.meas {
            let m = save_meas.meas;
            if m > measures {
                continue;
            }
            data[t][m] = save_meas.mml.clone();
        }
    }
}

pub(super) fn apply_save_file_to_track_volumes(
    file: &DawSaveFile,
    track_volumes_db: &mut [i32],
    tracks: usize,
) {
    for save_track in &file.tracks {
        let t = save_track.track;
        if t >= tracks || t < FIRST_PLAYABLE_TRACK {
            continue;
        }
        track_volumes_db[t] = save_track
            .volume_db
            .unwrap_or(0)
            .clamp(MIXER_MIN_DB, MIXER_MAX_DB);
    }
}

impl DawApp {
    // ─── 保存 / 読み込み ──────────────────────────────────────

    pub(super) fn load(&mut self) {
        let path = crate::history::daw_file_load_path();
        let content = path.as_ref().and_then(|p| std::fs::read_to_string(p).ok());
        if let Some(content) = content {
            if let Ok(file) = serde_json::from_str::<DawSaveFile>(&content) {
                // JSON が正常にパースできた場合は、ファイルが正式な保存データであるとみなす。
                // new() で設定したデフォルト値を残さないよう全セルをクリアしてから JSON の内容を適用する。
                // （空セルは JSON に含まれないため、クリアしないとデフォルト値が復活する）
                for row in &mut self.data {
                    for cell in row.iter_mut() {
                        cell.clear();
                    }
                }
                self.track_volumes_db.fill(0);
                apply_save_file_to_data(&file, &mut self.data, self.tracks, self.measures);
                apply_save_file_to_track_volumes(&file, &mut self.track_volumes_db, self.tracks);
            }
        }
        self.sync_cache_states();
        *self.play_track_gains.lock().unwrap() = self.playback_track_gains();
        let daw_state = crate::history::load_daw_session_state();
        self.cursor_track = daw_state.cursor_track.min(self.tracks - 1);
        self.cursor_measure = daw_state.cursor_measure.min(self.measures);
        self.restore_cache_from_history(&daw_state);
    }

    pub(super) fn save(&self) {
        let Some(path) = crate::history::daw_file_path() else {
            return;
        };
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let file = data_to_save_file(
            &self.data,
            &self.track_volumes_db,
            self.tracks,
            self.measures,
        );
        if let Ok(json) = serde_json::to_string_pretty(&file) {
            let _ = std::fs::write(&path, json);
        }
    }

    pub(super) fn save_history_state(&mut self) {
        self.flush_patch_phrase_store_if_dirty();
        let _ = crate::history::save_daw_session_state(&crate::history::DawSessionState {
            cursor_track: self.cursor_track,
            cursor_measure: self.cursor_measure,
            cached_measures: self.cached_measures_for_history(),
        });
    }
}

#[cfg(test)]
#[path = "../tests/daw/save.rs"]
mod tests;
