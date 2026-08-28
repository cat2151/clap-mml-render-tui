//! 明示的に保存・読み込みする DAW project file。

mod save;
mod validate;

use validate::validate_project_file;
pub(crate) use validate::{project_snapshot_for_recovery, validate_project_file_for_recovery};

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use super::{
    mml::{build_cell_mml_from_data, cell_has_content, measure_duration_samples_from_data},
    tracks::{
        grid_row_from_saved_track, grid_track_count_from_saved, saved_track_count_from_grid,
        saved_track_from_grid_row, CHORD_TRACK, FIRST_SAVED_PLAYABLE_TRACK,
    },
    AbRepeatState, CacheState, CellCache, DawApp, FIRST_PLAYABLE_TRACK, MIXER_MAX_DB, MIXER_MIN_DB,
};

const PROJECT_FORMAT: &str = "clap-mml-render-tui.daw-project";
const PROJECT_FORMAT_VERSION: u32 = 1;
pub(crate) const DEFAULT_PROJECT_FILE_NAME: &str = "project.cmrt-daw.json";

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct DawProjectFile {
    format: String,
    format_version: u32,
    project: DawProjectData,
}

/// project file 上の track 表現。
///
/// `track_count` / `track_index` は chord 行が入る前の番号のまま
/// （0 = global header, 1.. = instrument）。chord 行だけは `chord_track` に置く。
/// これで chord 行を使わない project file は従来と同一内容のまま読み書きできる。
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DawProjectData {
    track_count: usize,
    playable_measure_count: usize,
    tracks: Vec<DawProjectTrack>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    chord_track: Option<DawProjectChordTrack>,
}

/// project file 上の chord 行。全セルが空なら書き出さない。
#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DawProjectChordTrack {
    non_empty_cells: Vec<DawProjectCell>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DawProjectTrackRole {
    GlobalHeader,
    Instrument,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DawProjectTrack {
    track_index: usize,
    role: DawProjectTrackRole,
    volume_db: i32,
    non_empty_cells: Vec<DawProjectCell>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
enum DawProjectCellRole {
    Initialization,
    PlayableMeasure,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DawProjectCell {
    measure_index: usize,
    role: DawProjectCellRole,
    mml: String,
}

pub(crate) struct DawProjectSnapshot {
    data: Vec<Vec<String>>,
    track_volumes_db: Vec<i32>,
    tracks: usize,
    measures: usize,
}

pub(crate) struct DawProjectPreview {
    pub(crate) tracks: usize,
    pub(crate) measures: usize,
    pub(crate) measure_index: Option<usize>,
    pub(crate) measure_samples: usize,
    pub(crate) track_mmls: Vec<String>,
    pub(crate) track_gains: Vec<f32>,
}

pub(crate) struct DawProjectSaveResult {
    pub(crate) path: PathBuf,
    pub(crate) backup_path: Option<PathBuf>,
}

fn track_role(track: usize) -> DawProjectTrackRole {
    if track == 0 {
        DawProjectTrackRole::GlobalHeader
    } else {
        DawProjectTrackRole::Instrument
    }
}

fn cell_role(measure: usize) -> DawProjectCellRole {
    if measure == 0 {
        DawProjectCellRole::Initialization
    } else {
        DawProjectCellRole::PlayableMeasure
    }
}

fn non_empty_cells(row: &[String], measures: usize) -> Vec<DawProjectCell> {
    row.iter()
        .enumerate()
        .take(measures + 1)
        .filter(|(_, mml)| !mml.trim().is_empty())
        .map(|(measure_index, mml)| DawProjectCell {
            measure_index,
            role: cell_role(measure_index),
            mml: mml.clone(),
        })
        .collect()
}

pub(crate) fn project_file_from_app(app: &DawApp) -> DawProjectFile {
    let tracks = app
        .editor
        .data
        .iter()
        .enumerate()
        .take(app.editor.tracks)
        .filter_map(|(row_index, row)| {
            let track_index = saved_track_from_grid_row(row_index)?;
            Some(DawProjectTrack {
                track_index,
                role: track_role(track_index),
                volume_db: app.track_volumes_db.get(row_index).copied().unwrap_or(0),
                non_empty_cells: non_empty_cells(row, app.editor.measures),
            })
        })
        .collect();
    let chord_cells = app
        .editor
        .data
        .get(CHORD_TRACK)
        .map(|row| non_empty_cells(row, app.editor.measures))
        .unwrap_or_default();

    DawProjectFile {
        format: PROJECT_FORMAT.to_string(),
        format_version: PROJECT_FORMAT_VERSION,
        project: DawProjectData {
            track_count: saved_track_count_from_grid(app.editor.tracks),
            playable_measure_count: app.editor.measures,
            tracks,
            chord_track: (!chord_cells.is_empty()).then_some(DawProjectChordTrack {
                non_empty_cells: chord_cells,
            }),
        },
    }
}

fn resolve_project_path(path_text: &str) -> Result<PathBuf> {
    let path_text = path_text.trim();
    if path_text.is_empty() {
        bail!("project file path を入力してください");
    }
    let path = PathBuf::from(path_text);
    if path.is_absolute() {
        Ok(path)
    } else {
        Ok(std::env::current_dir()
            .context("current directory を取得できません")?
            .join(path))
    }
}

fn read_project_file(path: &Path) -> Result<DawProjectSnapshot> {
    let json = std::fs::read_to_string(path)
        .with_context(|| format!("project file を読めません: {}", path.display()))?;
    let file = serde_json::from_str::<DawProjectFile>(&json)
        .with_context(|| format!("project JSON を解釈できません: {}", path.display()))?;
    validate_project_file(&file)
}

fn preview_from_snapshot(snapshot: &DawProjectSnapshot, sample_rate: f64) -> DawProjectPreview {
    // 「中身のある最初の小節」は生のセル文字列では判定しない。
    // chord 行から生成される track はセルが空のままでも音が出る。
    let measure = (1..=snapshot.measures).find(|&measure| {
        (FIRST_PLAYABLE_TRACK..snapshot.tracks)
            .any(|track| cell_has_content(&snapshot.data, track, measure))
    });
    let track_mmls = (0..snapshot.tracks)
        .map(|track| match measure {
            Some(measure)
                if track >= FIRST_PLAYABLE_TRACK
                    && cell_has_content(&snapshot.data, track, measure) =>
            {
                build_cell_mml_from_data(&snapshot.data, snapshot.measures, track, measure)
            }
            _ => String::new(),
        })
        .collect();
    let track_gains = snapshot
        .track_volumes_db
        .iter()
        .enumerate()
        .map(|(track, volume_db)| {
            if track < FIRST_PLAYABLE_TRACK {
                0.0
            } else {
                10.0f32.powf(*volume_db as f32 / 20.0)
            }
        })
        .collect();

    DawProjectPreview {
        tracks: snapshot.tracks,
        measures: snapshot.measures,
        measure_index: measure.map(|measure| measure - 1),
        measure_samples: measure_duration_samples_from_data(
            &snapshot.data,
            snapshot.measures,
            sample_rate,
        ),
        track_mmls,
        track_gains,
    }
}

impl DawApp {
    pub(crate) fn inspect_project_for_preview(&self, path: &Path) -> Result<DawProjectPreview> {
        let snapshot = read_project_file(path)?;
        Ok(preview_from_snapshot(&snapshot, self.cfg.sample_rate))
    }

    pub(crate) fn save_project_as(&self, path_text: &str) -> Result<DawProjectSaveResult> {
        let path = resolve_project_path(path_text)?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).with_context(|| {
                format!("project directory を作成できません: {}", parent.display())
            })?;
        }
        let mut json = serde_json::to_string_pretty(&project_file_from_app(self))
            .context("project JSON を生成できません")?;
        json.push('\n');
        let backup_path = save::write_project_file(&path, json.as_bytes())?;
        Ok(DawProjectSaveResult { path, backup_path })
    }

    pub(crate) fn open_project(&mut self, path_text: &str) -> Result<PathBuf> {
        let path = resolve_project_path(path_text)?;
        let snapshot = read_project_file(&path)?;
        self.apply_project_snapshot(snapshot);
        Ok(path)
    }

    pub(crate) fn apply_project_snapshot_for_recovery(&mut self, snapshot: DawProjectSnapshot) {
        self.apply_project_snapshot_state(snapshot);
    }

    fn apply_project_snapshot(&mut self, snapshot: DawProjectSnapshot) {
        self.apply_project_snapshot_state(snapshot);
        self.save();
        self.kick_all_pending();
    }

    fn apply_project_snapshot_state(&mut self, snapshot: DawProjectSnapshot) {
        self.stop_play();

        let columns = snapshot.measures + 1;
        {
            let mut cache = self.cache.lock().unwrap();
            // In-flight jobs from the previous project must not be accepted for the new one.
            for row in cache.iter_mut() {
                for cell in row.iter_mut() {
                    cell.generation = cell.generation.wrapping_add(1);
                    if cell.generation == 0 {
                        cell.generation = 1;
                    }
                    cell.state = CacheState::Empty;
                    cell.samples = None;
                    cell.rendered_measure_samples = None;
                    cell.rendered_mml_hash = None;
                }
            }
            cache.resize_with(snapshot.tracks, || vec![CellCache::empty(); columns]);
            cache.truncate(snapshot.tracks);
            for row in cache.iter_mut() {
                row.resize_with(columns, CellCache::empty);
                row.truncate(columns);
            }
        }

        self.editor.data = snapshot.data;
        self.editor.tracks = snapshot.tracks;
        self.editor.measures = snapshot.measures;
        self.editor.cursor_track = self.editor.cursor_track.min(snapshot.tracks - 1);
        self.editor.cursor_measure = self.editor.cursor_measure.min(snapshot.measures);
        self.editor.yank_buffer = None;
        self.editor.pending_delete = false;
        self.editor.cell_undo = None;
        self.editor.chord_jump_return_track = None;
        self.track_volumes_db = snapshot.track_volumes_db;
        self.solo_tracks = vec![false; snapshot.tracks];
        *self.track_rerender_batches.lock().unwrap() = (0..snapshot.tracks).map(|_| None).collect();
        *self.playback.ab_repeat.lock().unwrap() = AbRepeatState::Off;
        self.playback.overlay_preview_cache.lock().unwrap().clear();
        self.overlays.mixer.cursor_track = self
            .overlays
            .mixer
            .cursor_track
            .clamp(FIRST_PLAYABLE_TRACK, snapshot.tracks - 1);

        self.sync_cache_states();
        self.sync_playback_mml_state();
        self.sync_http_grid_snapshot();
    }
}

#[cfg(test)]
mod tests;
