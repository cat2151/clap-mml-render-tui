use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::loop_browser_metadata::{validate_wav_id, LoopWavId};

const TRACK_GRID_VERSION: u32 = 2;
const TRACK_GRID_DIRECTORY: &str = "loop_browser";
const TRACK_GRID_FILE_NAME: &str = "track_grid.toml";

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LoopTrackClip {
    pub(crate) wav: LoopWavId,
    pub(crate) span_measures: usize,
}

pub(crate) type LoopTrackGrid = Vec<Vec<Option<LoopTrackClip>>>;

pub(crate) struct LoadedTrackGrid {
    pub(crate) grid: LoopTrackGrid,
    pub(crate) needs_migration: bool,
}

#[derive(Debug, Deserialize, Serialize)]
struct StoredTrackGrid {
    version: u32,
    track_count: usize,
    measure_count: usize,
    #[serde(default)]
    cells: Vec<StoredTrackCell>,
}

#[derive(Debug, Deserialize, Serialize)]
struct StoredTrackCell {
    track: usize,
    measure: usize,
    #[serde(default = "default_span")]
    span_measures: usize,
    #[serde(flatten)]
    wav: LoopWavId,
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
            needs_migration: false,
        });
    }
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("track gridを読めません: {}", path.display()))?;
    let stored: StoredTrackGrid = toml::from_str(&text)
        .with_context(|| format!("track gridが壊れています: {}", path.display()))?;
    let needs_migration = stored.version == 1;
    let grid = stored.into_grid()?;
    Ok(LoadedTrackGrid {
        grid,
        needs_migration,
    })
}

pub(crate) fn save_to(path: &Path, grid: &LoopTrackGrid) -> Result<()> {
    let stored = StoredTrackGrid::from_grid(grid)?;
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
    fn from_grid(grid: &LoopTrackGrid) -> Result<Self> {
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
                cells.push(StoredTrackCell {
                    track,
                    measure,
                    span_measures: clip.span_measures,
                    wav: clip.wav.clone(),
                });
            }
        }
        Ok(Self {
            version: TRACK_GRID_VERSION,
            track_count: grid.len(),
            measure_count,
            cells,
        })
    }

    fn into_grid(self) -> Result<LoopTrackGrid> {
        if !matches!(self.version, 1 | TRACK_GRID_VERSION) {
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
        for cell in self.cells {
            if cell.track >= self.track_count || cell.measure >= self.measure_count {
                anyhow::bail!(
                    "track gridのcell位置が範囲外です: track={}, measure={}",
                    cell.track,
                    cell.measure
                );
            }
            validate_wav_id(&cell.wav)?;
            if cell.span_measures == 0 {
                anyhow::bail!("track gridのclip spanは1以上である必要があります");
            }
            let destination = &mut grid[cell.track][cell.measure];
            if destination.is_some() {
                anyhow::bail!(
                    "track gridに重複したcellがあります: track={}, measure={}",
                    cell.track,
                    cell.measure
                );
            }
            *destination = Some(LoopTrackClip {
                wav: cell.wav,
                span_measures: cell.span_measures,
            });
        }
        Ok(grid)
    }
}

pub(crate) fn reflow_with_spans(
    grid: &LoopTrackGrid,
    mut span_for: impl FnMut(&LoopWavId) -> Option<usize>,
) -> (LoopTrackGrid, bool) {
    let original_width = grid.first().map_or(1, Vec::len).max(1);
    let mut tracks = Vec::with_capacity(grid.len().max(1));
    let mut width = original_width;
    let mut changed = false;
    for track in grid {
        let mut updated = vec![None; original_width];
        let mut occupied_until = 0;
        for (old_measure, clip) in track
            .iter()
            .enumerate()
            .filter_map(|(measure, clip)| clip.as_ref().map(|clip| (measure, clip)))
        {
            let span = span_for(&clip.wav).unwrap_or(clip.span_measures).max(1);
            let measure = old_measure.max(occupied_until);
            let end = measure.saturating_add(span);
            if end > updated.len() {
                updated.resize(end, None);
            }
            updated[measure] = Some(LoopTrackClip {
                wav: clip.wav.clone(),
                span_measures: span,
            });
            occupied_until = end;
            width = width.max(end);
            changed |= measure != old_measure || span != clip.span_measures;
        }
        tracks.push(updated);
    }
    if tracks.is_empty() {
        tracks.push(vec![None; width]);
        changed = true;
    }
    for track in &mut tracks {
        track.resize(width, None);
    }
    changed |= grid.iter().any(|track| track.len() != width);
    (tracks, changed)
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
