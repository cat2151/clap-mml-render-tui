//! DAW セッションの保存・読み込み

use super::DawApp;

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

/// data グリッドを `DawSaveFile` に変換する（空トラック・空小節は除外）。
pub(super) fn data_to_save_file(
    data: &[Vec<String>],
    tracks: usize,
    measures: usize,
) -> DawSaveFile {
    let mut save_tracks: Vec<DawSaveTrack> = Vec::new();
    for t in 0..tracks {
        let mut save_meas: Vec<DawSaveMeas> = Vec::new();
        for m in 0..=measures {
            if !data[t][m].trim().is_empty() {
                let description = if m == 0 {
                    Some("initial".to_string())
                } else {
                    None
                };
                save_meas.push(DawSaveMeas {
                    meas: m,
                    description,
                    mml: data[t][m].clone(),
                });
            }
        }
        if !save_meas.is_empty() {
            let description = if t == 0 {
                Some("tempo track".to_string())
            } else {
                None
            };
            save_tracks.push(DawSaveTrack {
                track: t,
                description,
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
    data: &mut Vec<Vec<String>>,
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

impl DawApp {
    // ─── 保存 / 読み込み ──────────────────────────────────────

    pub(super) fn load(&mut self) {
        let path = crate::history::daw_file_path();
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
                apply_save_file_to_data(&file, &mut self.data, self.tracks, self.measures);
            }
        }
        self.sync_cache_states();
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
        let file = data_to_save_file(&self.data, self.tracks, self.measures);
        if let Ok(json) = serde_json::to_string_pretty(&file) {
            let _ = std::fs::write(&path, json);
        }
    }

    pub(super) fn save_history_state(&self) {
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
