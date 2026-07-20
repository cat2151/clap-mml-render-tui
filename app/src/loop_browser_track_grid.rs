use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

use crate::loop_browser_metadata::{validate_wav_id, LoopWavId};

const TRACK_GRID_VERSION: u32 = 1;
const TRACK_GRID_DIRECTORY: &str = "loop_browser";
const TRACK_GRID_FILE_NAME: &str = "track_grid.toml";

pub(crate) type LoopTrackGrid = Vec<Vec<Option<LoopWavId>>>;

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
    #[serde(flatten)]
    wav: LoopWavId,
}

pub(crate) fn default_track_grid() -> LoopTrackGrid {
    vec![vec![None]]
}

pub(crate) fn track_grid_path() -> Result<PathBuf> {
    crate::config::config_app_dir()
        .map(|dir| dir.join(TRACK_GRID_DIRECTORY).join(TRACK_GRID_FILE_NAME))
        .ok_or_else(|| anyhow::anyhow!("システムの設定ディレクトリが取得できません"))
}

pub(crate) fn load_from(path: &Path) -> Result<LoopTrackGrid> {
    if !path.exists() {
        return Ok(default_track_grid());
    }
    let text = std::fs::read_to_string(path)
        .with_context(|| format!("track gridを読めません: {}", path.display()))?;
    let stored: StoredTrackGrid = toml::from_str(&text)
        .with_context(|| format!("track gridが壊れています: {}", path.display()))?;
    stored.into_grid()
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
            for (measure, wav) in measures.iter().enumerate() {
                let Some(wav) = wav else {
                    continue;
                };
                validate_wav_id(wav)?;
                cells.push(StoredTrackCell {
                    track,
                    measure,
                    wav: wav.clone(),
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
        if self.version != TRACK_GRID_VERSION {
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
            let destination = &mut grid[cell.track][cell.measure];
            if destination.is_some() {
                anyhow::bail!(
                    "track gridに重複したcellがあります: track={}, measure={}",
                    cell.track,
                    cell.measure
                );
            }
            *destination = Some(cell.wav);
        }
        Ok(grid)
    }
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
