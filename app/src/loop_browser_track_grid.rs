use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::loop_browser_metadata::{validate_wav_id, LoopWavId};

mod normalize;

pub(crate) use normalize::{normalize_previous_markers, reflow_with_spans};

const TRACK_GRID_VERSION: u32 = 4;
const TRACK_GRID_DIRECTORY: &str = "loop_browser";
const TRACK_GRID_FILE_NAME: &str = "track_grid.toml";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LoopTrackClip {
    pub(crate) wav: LoopWavId,
    pub(crate) span_measures: usize,
    pub(crate) previous_source_measure: Option<usize>,
}

impl LoopTrackClip {
    pub(crate) fn explicit(wav: LoopWavId, span_measures: usize) -> Self {
        Self {
            wav,
            span_measures,
            previous_source_measure: None,
        }
    }

    pub(crate) fn is_previous(&self) -> bool {
        self.previous_source_measure.is_some()
    }
}

pub(crate) type LoopTrackGrid = Vec<Vec<Option<LoopTrackClip>>>;

pub(crate) struct LoadedTrackGrid {
    pub(crate) grid: LoopTrackGrid,
    pub(crate) track_volumes_db: Vec<i32>,
    pub(crate) needs_migration: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct StoredTrackGrid {
    version: u32,
    track_count: usize,
    measure_count: usize,
    #[serde(default)]
    track_volumes_db: Vec<i32>,
    #[serde(default)]
    cells: Vec<StoredTrackCell>,
}

#[derive(Debug, Deserialize, Serialize)]
struct StoredTrackCell {
    track: usize,
    measure: usize,
    #[serde(default = "default_span")]
    span_measures: usize,
    #[serde(default, skip_serializing_if = "StoredTrackCellKind::is_clip")]
    kind: StoredTrackCellKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    source_measure: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    root: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    relative: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum StoredTrackCellKind {
    #[default]
    Clip,
    Prev,
}

impl StoredTrackCellKind {
    fn is_clip(&self) -> bool {
        *self == Self::Clip
    }
}

const fn default_span() -> usize {
    1
}

pub(crate) fn default_track_grid() -> LoopTrackGrid {
    vec![vec![None]]
}

pub(crate) fn track_grid_path() -> Result<PathBuf> {
    crate::config::config_app_dir()
        .map(|dir| dir.join(TRACK_GRID_DIRECTORY).join(TRACK_GRID_FILE_NAME))
        .ok_or_else(|| anyhow::anyhow!("システムの設定ディレクトリが取得できません"))
}

pub(crate) fn load_from(path: &Path) -> Result<LoadedTrackGrid> {
    if !path.exists() {
        return Ok(LoadedTrackGrid {
            grid: default_track_grid(),
            track_volumes_db: vec![0],
            needs_migration: false,
        });
    }
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("track gridを読めません: {}", path.display()))?;
    let stored: StoredTrackGrid = toml::from_str(&text)
        .with_context(|| format!("track gridが壊れています: {}", path.display()))?;
    let mut track_volumes_db = stored.track_volumes_db.clone();
    let mut needs_migration =
        stored.version < TRACK_GRID_VERSION || track_volumes_db.len() != stored.track_count;
    track_volumes_db.resize(stored.track_count, 0);
    track_volumes_db.truncate(stored.track_count);
    for volume_db in &mut track_volumes_db {
        let normalized = normalize_volume_db(*volume_db);
        needs_migration |= normalized != *volume_db;
        *volume_db = normalized;
    }
    let grid = stored.into_grid()?;
    Ok(LoadedTrackGrid {
        grid,
        track_volumes_db,
        needs_migration,
    })
}

pub(crate) fn save_to(path: &Path, grid: &LoopTrackGrid, track_volumes_db: &[i32]) -> Result<()> {
    let stored = StoredTrackGrid::from_grid(grid, track_volumes_db)?;
    let text = toml::to_string_pretty(&stored)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("track grid directoryを作れません: {}", parent.display()))?;
    }
    let temp_path = unique_temp_path(path);
    let write_result = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp_path)
        .and_then(|mut file| {
            use std::io::Write as _;
            file.write_all(text.as_bytes())?;
            file.sync_all()
        });
    if let Err(error) = write_result {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error)
            .with_context(|| format!("一時track gridを書けません: {}", temp_path.display()));
    }
    if let Err(error) = replace_file(&temp_path, path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error)
            .with_context(|| format!("track gridを置換できません: {}", path.display()));
    }
    Ok(())
}

impl StoredTrackGrid {
    fn from_grid(grid: &LoopTrackGrid, track_volumes_db: &[i32]) -> Result<Self> {
        let Some(first_track) = grid.first() else {
            anyhow::bail!("track gridにtrackがありません");
        };
        let measure_count = first_track.len();
        if measure_count == 0 {
            anyhow::bail!("track gridにmeasureがありません");
        }
        if grid.iter().any(|track| track.len() != measure_count) {
            anyhow::bail!("track gridのmeasure数が揃っていません");
        }
        if track_volumes_db.len() != grid.len() {
            anyhow::bail!("track gridのtrack数とmix level数が一致しません");
        }
        if track_volumes_db
            .iter()
            .any(|volume_db| normalize_volume_db(*volume_db) != *volume_db)
        {
            anyhow::bail!("track gridのmix levelが3dB単位または範囲内ではありません");
        }
        let mut cells = Vec::new();
        for (track, measures) in grid.iter().enumerate() {
            let mut occupied_until = 0;
            for (measure, clip) in measures.iter().enumerate() {
                let Some(clip) = clip else {
                    continue;
                };
                validate_wav_id(&clip.wav)?;
                if clip.span_measures == 0 || measure < occupied_until {
                    anyhow::bail!("track gridに不正または重複したclip spanがあります");
                }
                occupied_until = measure
                    .checked_add(clip.span_measures)
                    .ok_or_else(|| anyhow::anyhow!("track gridのclip spanが大きすぎます"))?;
                if occupied_until > measure_count {
                    anyhow::bail!("track gridのclipがmeasure範囲外です");
                }
                let (kind, source_measure, root, relative) = if let Some(source_measure) =
                    clip.previous_source_measure
                {
                    let source = measures
                            .get(source_measure)
                            .and_then(Option::as_ref)
                            .filter(|source| !source.is_previous())
                            .ok_or_else(|| {
                                anyhow::anyhow!(
                                    "prev markerの参照先が通常clipではありません: track={track}, measure={measure}, source_measure={source_measure}"
                                )
                            })?;
                    if !source.wav.matches(&clip.wav) {
                        anyhow::bail!(
                                "prev markerのWAVが参照先と一致しません: track={track}, measure={measure}"
                            );
                    }
                    (StoredTrackCellKind::Prev, Some(source_measure), None, None)
                } else {
                    (
                        StoredTrackCellKind::Clip,
                        None,
                        Some(clip.wav.root.clone()),
                        Some(clip.wav.relative.clone()),
                    )
                };
                cells.push(StoredTrackCell {
                    track,
                    measure,
                    span_measures: clip.span_measures,
                    kind,
                    source_measure,
                    root,
                    relative,
                });
            }
        }
        Ok(Self {
            version: TRACK_GRID_VERSION,
            track_count: grid.len(),
            measure_count,
            track_volumes_db: track_volumes_db.to_vec(),
            cells,
        })
    }

    fn into_grid(self) -> Result<LoopTrackGrid> {
        if !matches!(self.version, 1 | 2 | 3 | TRACK_GRID_VERSION) {
            anyhow::bail!(
                "track gridのversionが一致しません（file: {}, expected: {}）",
                self.version,
                TRACK_GRID_VERSION
            );
        }
        if self.track_count == 0 || self.measure_count == 0 {
            anyhow::bail!("track gridのtrack数とmeasure数は1以上である必要があります");
        }
        let mut grid = vec![vec![None; self.measure_count]; self.track_count];
        let mut previous_cells = Vec::new();
        let mut positions = HashSet::new();
        for cell in self.cells {
            if cell.track >= self.track_count || cell.measure >= self.measure_count {
                anyhow::bail!(
                    "track gridのcell位置が範囲外です: track={}, measure={}",
                    cell.track,
                    cell.measure
                );
            }
            if cell.span_measures == 0 {
                anyhow::bail!("track gridのclip spanは1以上である必要があります");
            }
            if !positions.insert((cell.track, cell.measure)) {
                anyhow::bail!(
                    "track gridに重複したcellがあります: track={}, measure={}",
                    cell.track,
                    cell.measure
                );
            }
            let destination = &mut grid[cell.track][cell.measure];
            match cell.kind {
                StoredTrackCellKind::Clip => {
                    let wav = LoopWavId {
                        root: cell
                            .root
                            .ok_or_else(|| anyhow::anyhow!("通常clipにrootがありません"))?,
                        relative: cell
                            .relative
                            .ok_or_else(|| anyhow::anyhow!("通常clipにrelativeがありません"))?,
                    };
                    validate_wav_id(&wav)?;
                    *destination = Some(LoopTrackClip::explicit(wav, cell.span_measures));
                }
                StoredTrackCellKind::Prev => {
                    if cell.root.is_some() || cell.relative.is_some() {
                        anyhow::bail!("prev markerにWAV pathは保存できません");
                    }
                    let source_measure = cell.source_measure.ok_or_else(|| {
                        anyhow::anyhow!("prev markerにsource_measureがありません")
                    })?;
                    previous_cells.push((
                        cell.track,
                        cell.measure,
                        cell.span_measures,
                        source_measure,
                    ));
                }
            }
        }
        for (track, measure, span_measures, source_measure) in previous_cells {
            let source_wav = grid
                .get(track)
                .and_then(|cells| cells.get(source_measure))
                .and_then(Option::as_ref)
                .filter(|source| !source.is_previous())
                .map(|source| source.wav.clone())
                .ok_or_else(|| {
                    anyhow::anyhow!(
                        "prev markerの参照先が通常clipではありません: track={track}, measure={measure}, source_measure={source_measure}"
                    )
                })?;
            grid[track][measure] = Some(LoopTrackClip {
                wav: source_wav,
                span_measures,
                previous_source_measure: Some(source_measure),
            });
        }
        validate_no_overlaps(&grid)?;
        Ok(grid)
    }
}

fn validate_no_overlaps(grid: &LoopTrackGrid) -> Result<()> {
    for measures in grid {
        let mut occupied_until = 0;
        for (measure, clip) in measures.iter().enumerate() {
            let Some(clip) = clip else {
                continue;
            };
            if measure < occupied_until {
                anyhow::bail!("track gridに重複したclip spanがあります");
            }
            occupied_until = measure
                .checked_add(clip.span_measures)
                .ok_or_else(|| anyhow::anyhow!("track gridのclip spanが大きすぎます"))?;
            if occupied_until > measures.len() {
                anyhow::bail!("track gridのclipがmeasure範囲外です");
            }
        }
    }
    Ok(())
}

fn normalize_volume_db(volume_db: i32) -> i32 {
    let clamped = volume_db.clamp(
        crate::mixer_overlay::MIXER_MIN_DB,
        crate::mixer_overlay::MIXER_MAX_DB,
    );
    ((clamped as f32 / crate::mixer_overlay::MIXER_STEP_DB as f32).round() as i32
        * crate::mixer_overlay::MIXER_STEP_DB)
        .clamp(
            crate::mixer_overlay::MIXER_MIN_DB,
            crate::mixer_overlay::MIXER_MAX_DB,
        )
}

fn unique_temp_path(path: &Path) -> PathBuf {
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    path.with_extension(format!("tmp-{}-{nonce}", std::process::id()))
}

#[cfg(windows)]
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt as _;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };
    let from = from
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let to = to
        .as_os_str()
        .encode_wide()
        .chain(std::iter::once(0))
        .collect::<Vec<_>>();
    let result = unsafe {
        MoveFileExW(
            from.as_ptr(),
            to.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(std::io::Error::last_os_error())
    } else {
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    std::fs::rename(from, to)
}

#[cfg(test)]
#[path = "tests/loop_browser_track_grid.rs"]
mod tests;
