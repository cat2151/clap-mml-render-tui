//! 明示的に保存・読み込みする DAW project file。

mod save;

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

use super::{
    mml::{build_cell_mml_from_data, measure_duration_samples_from_data},
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct DawProjectData {
    track_count: usize,
    playable_measure_count: usize,
    tracks: Vec<DawProjectTrack>,
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

pub(crate) fn project_file_from_app(app: &DawApp) -> DawProjectFile {
    let tracks = app
        .editor
        .data
        .iter()
        .enumerate()
        .take(app.editor.tracks)
        .map(|(track_index, row)| DawProjectTrack {
            track_index,
            role: track_role(track_index),
            volume_db: app.track_volumes_db.get(track_index).copied().unwrap_or(0),
            non_empty_cells: row
                .iter()
                .enumerate()
                .take(app.editor.measures + 1)
                .filter(|(_, mml)| !mml.trim().is_empty())
                .map(|(measure_index, mml)| DawProjectCell {
                    measure_index,
                    role: cell_role(measure_index),
                    mml: mml.clone(),
                })
                .collect(),
        })
        .collect();

    DawProjectFile {
        format: PROJECT_FORMAT.to_string(),
        format_version: PROJECT_FORMAT_VERSION,
        project: DawProjectData {
            track_count: app.editor.tracks,
            playable_measure_count: app.editor.measures,
            tracks,
        },
    }
}

fn try_empty_grid(tracks: usize, measures: usize) -> Result<Vec<Vec<String>>> {
    let columns = measures
        .checked_add(1)
        .context("playable_measure_count が大きすぎます")?;
    tracks
        .checked_mul(columns)
        .context("project grid が大きすぎます")?;

    let mut data = Vec::new();
    data.try_reserve_exact(tracks)
        .context("project track 用メモリを確保できません")?;
    for _ in 0..tracks {
        let mut row = Vec::new();
        row.try_reserve_exact(columns)
            .context("project measure 用メモリを確保できません")?;
        row.resize_with(columns, String::new);
        data.push(row);
    }
    Ok(data)
}

fn validate_project_file(file: &DawProjectFile) -> Result<DawProjectSnapshot> {
    if file.format != PROJECT_FORMAT {
        bail!(
            "project format が違います: expected={PROJECT_FORMAT}, actual={}",
            file.format
        );
    }
    if file.format_version != PROJECT_FORMAT_VERSION {
        bail!(
            "未対応の project format_version です: {}",
            file.format_version
        );
    }

    let project = &file.project;
    if project.track_count <= FIRST_PLAYABLE_TRACK {
        bail!("track_count は2以上である必要があります");
    }
    if project.playable_measure_count == 0 {
        bail!("playable_measure_count は1以上である必要があります");
    }
    if project.tracks.len() != project.track_count {
        bail!(
            "tracks の要素数が track_count と一致しません: {} != {}",
            project.tracks.len(),
            project.track_count
        );
    }

    let mut data = try_empty_grid(project.track_count, project.playable_measure_count)?;
    let mut track_volumes_db = Vec::new();
    track_volumes_db
        .try_reserve_exact(project.track_count)
        .context("track volume 用メモリを確保できません")?;
    track_volumes_db.resize(project.track_count, 0);

    for (expected_track_index, track) in project.tracks.iter().enumerate() {
        if track.track_index != expected_track_index {
            bail!(
                "tracks は track_index 順に重複なく並べる必要があります: expected={}, actual={}",
                expected_track_index,
                track.track_index
            );
        }
        if track.role != track_role(track.track_index) {
            bail!("track{} の role が index と一致しません", track.track_index);
        }
        if track.track_index == 0 && track.volume_db != 0 {
            bail!("global header track の volume_db は0である必要があります");
        }
        if !(MIXER_MIN_DB..=MIXER_MAX_DB).contains(&track.volume_db) {
            bail!(
                "track{} の volume_db が範囲外です: {}",
                track.track_index,
                track.volume_db
            );
        }
        track_volumes_db[track.track_index] = track.volume_db;

        let mut previous_measure = None;
        for cell in &track.non_empty_cells {
            if cell.measure_index > project.playable_measure_count {
                bail!(
                    "track{} の measure_index が範囲外です: {}",
                    track.track_index,
                    cell.measure_index
                );
            }
            if previous_measure.is_some_and(|previous| cell.measure_index <= previous) {
                bail!(
                    "track{} の non_empty_cells は measure_index 順に重複なく並べる必要があります",
                    track.track_index
                );
            }
            if cell.role != cell_role(cell.measure_index) {
                bail!(
                    "track{} measure{} の role が index と一致しません",
                    track.track_index,
                    cell.measure_index
                );
            }
            if cell.mml.trim().is_empty() {
                bail!(
                    "track{} measure{} は non_empty_cells 内で空にできません",
                    track.track_index,
                    cell.measure_index
                );
            }
            previous_measure = Some(cell.measure_index);
            data[track.track_index][cell.measure_index] = cell.mml.clone();
        }
    }

    Ok(DawProjectSnapshot {
        data,
        track_volumes_db,
        tracks: project.track_count,
        measures: project.playable_measure_count,
    })
}

pub(crate) fn validate_project_file_for_recovery(file: &DawProjectFile) -> Result<()> {
    validate_project_file(file).map(|_| ())
}

pub(crate) fn project_snapshot_for_recovery(file: &DawProjectFile) -> Result<DawProjectSnapshot> {
    validate_project_file(file)
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
    let measure = (1..=snapshot.measures).find(|&measure| {
        (FIRST_PLAYABLE_TRACK..snapshot.tracks).any(|track| {
            snapshot.data[track]
                .get(measure)
                .is_some_and(|mml| !mml.trim().is_empty())
        })
    });
    let track_mmls = (0..snapshot.tracks)
        .map(|track| {
            if track < FIRST_PLAYABLE_TRACK
                || measure.is_none_or(|measure| snapshot.data[track][measure].trim().is_empty())
            {
                String::new()
            } else {
                build_cell_mml_from_data(&snapshot.data, snapshot.measures, track, measure.unwrap())
            }
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
        self.editor.paste_undo = None;
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
