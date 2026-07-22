//! WAV ループライブラリの走査と永続インデックス。

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

use crate::loop_wav_analysis::{analyze_file, LoopWavAnalysis, LoopWavKind};
use crate::loop_waveform::{LoopWaveform, WAVEFORM_BINS_PER_MEASURE};

pub(crate) const LOOP_INDEX_VERSION: u32 = 6;
const LOOP_INDEX_FILE_NAME: &str = "loop_index.json";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct LoopIndex {
    pub(crate) version: u32,
    pub(crate) roots: Vec<LoopRootIndex>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct LoopRootIndex {
    pub(crate) path: String,
    pub(crate) wav_files: Vec<LoopWavIndex>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct LoopWavIndex {
    pub(crate) relative: String,
    pub(crate) analysis: LoopWavAnalysis,
    pub(crate) waveform: LoopWaveform,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoopScanSummary {
    pub roots: usize,
    pub wav_files: usize,
    pub skipped_wav_files: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LoopScanProgress {
    Started {
        roots: usize,
    },
    Analyzing {
        current: usize,
        total: usize,
        path: PathBuf,
    },
    Visualizing {
        bin: usize,
        bins: usize,
    },
    Skipped {
        path: PathBuf,
        error: String,
    },
}

pub fn scan_and_save(cfg: &crate::config::Config) -> Result<LoopScanSummary> {
    scan_and_save_with_progress(cfg, |_| {})
}

pub fn scan_and_save_with_progress(
    cfg: &crate::config::Config,
    progress: impl FnMut(LoopScanProgress),
) -> Result<LoopScanSummary> {
    scan_dirs_and_save_with_progress(&cfg.loop_dirs, progress)
}

#[cfg(test)]
fn scan_dirs_and_save(loop_dirs: &[String]) -> Result<LoopScanSummary> {
    scan_dirs_and_save_with_progress(loop_dirs, |_| {})
}

fn scan_dirs_and_save_with_progress(
    loop_dirs: &[String],
    mut progress: impl FnMut(LoopScanProgress),
) -> Result<LoopScanSummary> {
    let (index, skipped_wav_files) = build_index(loop_dirs, &mut progress)?;
    let summary = LoopScanSummary {
        roots: index.roots.len(),
        wav_files: index.roots.iter().map(|root| root.wav_files.len()).sum(),
        skipped_wav_files,
    };
    save_index(&index)?;
    Ok(summary)
}

pub(crate) fn load_index(cfg: &crate::config::Config) -> Result<LoopIndex> {
    if cfg.loop_dirs.is_empty() {
        anyhow::bail!("loop_dirs が空です。config.toml にルートを設定してください");
    }
    let path = loop_index_path()?;
    let bytes = std::fs::read(&path)
        .with_context(|| format!("ループキャッシュを読めません: {}", path.display()))?;
    let index: LoopIndex = serde_json::from_slice(&bytes)
        .with_context(|| format!("ループキャッシュが壊れています: {}", path.display()))?;
    validate_index(&index, &cfg.loop_dirs)?;
    Ok(index)
}

pub(crate) fn loop_index_path() -> Result<PathBuf> {
    crate::config::config_app_dir()
        .map(|dir| dir.join("cache").join(LOOP_INDEX_FILE_NAME))
        .ok_or_else(|| anyhow::anyhow!("システムの設定ディレクトリが取得できません"))
}

struct PendingRoot {
    configured_root: String,
    root: PathBuf,
    wav_paths: Vec<String>,
}

fn build_index(
    loop_dirs: &[String],
    progress: &mut impl FnMut(LoopScanProgress),
) -> Result<(LoopIndex, usize)> {
    if loop_dirs.is_empty() {
        anyhow::bail!("loop_dirs が空です。config.toml にルートを設定してください");
    }

    progress(LoopScanProgress::Started {
        roots: loop_dirs.len(),
    });

    let mut pending_roots = Vec::with_capacity(loop_dirs.len());
    for configured_root in loop_dirs {
        let root = PathBuf::from(configured_root);
        if !root.is_dir() {
            anyhow::bail!(
                "loop_dirs のパスがディレクトリではありません: {}",
                root.display()
            );
        }
        let mut wav_paths = Vec::new();
        collect_wav_paths(&root, &root, &mut wav_paths)?;
        wav_paths.sort_by(|left, right| {
            left.to_lowercase()
                .cmp(&right.to_lowercase())
                .then_with(|| left.cmp(right))
        });
        pending_roots.push(PendingRoot {
            configured_root: configured_root.clone(),
            root,
            wav_paths,
        });
    }

    let total = pending_roots.iter().map(|root| root.wav_paths.len()).sum();
    let mut roots = Vec::with_capacity(pending_roots.len());
    let mut current = 0;
    let mut skipped_wav_files = 0;
    for pending in pending_roots {
        let mut wav_files = Vec::with_capacity(pending.wav_paths.len());
        for relative in pending.wav_paths {
            current += 1;
            let path = pending.root.join(&relative);
            progress(LoopScanProgress::Analyzing {
                current,
                total,
                path: path.clone(),
            });
            match analyze_file(&path).and_then(|analysis| {
                let analysis = if is_one_shot_relative_path(&relative) {
                    analysis.into_one_shot()
                } else {
                    analysis
                };
                let waveform = crate::loop_waveform::analyze_file_with_progress(
                    &path,
                    analysis.measures,
                    |bin, bins| {
                        progress(LoopScanProgress::Visualizing { bin, bins });
                    },
                )?;
                Ok((analysis, waveform))
            }) {
                Ok((analysis, waveform)) => wav_files.push(LoopWavIndex {
                    relative,
                    analysis,
                    waveform,
                }),
                Err(error) => {
                    skipped_wav_files += 1;
                    progress(LoopScanProgress::Skipped {
                        path,
                        error: format!("{error:#}"),
                    });
                }
            }
        }
        roots.push(LoopRootIndex {
            path: pending.configured_root,
            wav_files,
        });
    }

    Ok((
        LoopIndex {
            version: LOOP_INDEX_VERSION,
            roots,
        },
        skipped_wav_files,
    ))
}

fn collect_wav_paths(root: &Path, dir: &Path, output: &mut Vec<String>) -> Result<()> {
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("ディレクトリを走査できません: {}", dir.display()))?;
    for entry in entries {
        let entry = entry
            .with_context(|| format!("ディレクトリエントリを読めません: {}", dir.display()))?;
        let file_type = entry
            .file_type()
            .with_context(|| format!("ファイル種別を取得できません: {}", entry.path().display()))?;
        if file_type.is_symlink() {
            continue;
        }
        let path = entry.path();
        if file_type.is_dir() {
            collect_wav_paths(root, &path, output)?;
        } else if file_type.is_file() && is_wav_path(&path) {
            let relative = path
                .strip_prefix(root)
                .with_context(|| format!("ルート相対パスを作れません: {}", path.display()))?;
            output.push(relative.to_string_lossy().into_owned());
        }
    }
    Ok(())
}

fn is_wav_path(path: &Path) -> bool {
    path.extension()
        .and_then(OsStr::to_str)
        .is_some_and(|extension| extension.eq_ignore_ascii_case("wav"))
}

fn is_one_shot_relative_path(relative: &str) -> bool {
    Path::new(relative).components().any(|component| {
        matches!(component, Component::Normal(name) if name.to_string_lossy().eq_ignore_ascii_case("One Shots"))
    })
}

fn validate_index(index: &LoopIndex, configured_roots: &[String]) -> Result<()> {
    if index.version != LOOP_INDEX_VERSION {
        anyhow::bail!(
            "ループキャッシュのversionが一致しません（cache: {}, expected: {}）",
            index.version,
            LOOP_INDEX_VERSION
        );
    }
    let cached_roots = index
        .roots
        .iter()
        .map(|root| normalized_root(&root.path))
        .collect::<Vec<_>>();
    let configured_roots = configured_roots
        .iter()
        .map(|root| normalized_root(root))
        .collect::<Vec<_>>();
    if cached_roots != configured_roots {
        anyhow::bail!("loop_dirs とループキャッシュが一致しません");
    }
    for root in &index.roots {
        for wav in &root.wav_files {
            validate_relative_wav_path(&wav.relative)?;
            validate_analysis(wav.analysis)?;
            validate_waveform(&wav.waveform, wav.analysis.measures)?;
        }
    }
    Ok(())
}

fn validate_analysis(analysis: LoopWavAnalysis) -> Result<()> {
    if !analysis.duration_seconds.is_finite() || analysis.duration_seconds <= 0.0 {
        anyhow::bail!("ループキャッシュに不正なWAV解析結果があります");
    }
    match (analysis.kind, analysis.tempo) {
        (LoopWavKind::OneShot, None) if analysis.measures == 1 => {}
        (LoopWavKind::Loop, Some(tempo))
            if tempo.bpm.is_finite()
                && tempo.bpm > 0.0
                && tempo
                    .declared_bpm
                    .is_none_or(|bpm| bpm.is_finite() && bpm > 0.0)
                && tempo.beats > 0
                && tempo.meter_numerator > 0
                && tempo.meter_denominator > 0
                && analysis.measures > 0 => {}
        _ => anyhow::bail!("ループキャッシュに不正なWAV解析結果があります"),
    }
    Ok(())
}

fn validate_waveform(waveform: &LoopWaveform, measures: usize) -> Result<()> {
    let expected = measures
        .checked_mul(WAVEFORM_BINS_PER_MEASURE)
        .ok_or_else(|| anyhow::anyhow!("ループキャッシュの波形要素数が大きすぎます"))?;
    if waveform.rms_db_tenths.len() != expected || waveform.spectral_flux.len() != expected {
        anyhow::bail!(
            "ループキャッシュの波形要素数が不正です: rms={}, flux={}, expected={expected}",
            waveform.rms_db_tenths.len(),
            waveform.spectral_flux.len()
        );
    }
    if waveform
        .rms_db_tenths
        .iter()
        .any(|value| !(-960..=240).contains(value))
    {
        anyhow::bail!("ループキャッシュのRMS値が不正です");
    }
    Ok(())
}

fn normalized_root(root: &str) -> String {
    let normalized = Path::new(root).components().collect::<PathBuf>();
    let text = normalized.to_string_lossy().into_owned();
    if cfg!(windows) {
        text.to_lowercase()
    } else {
        text
    }
}

fn validate_relative_wav_path(relative: &str) -> Result<()> {
    let path = Path::new(relative);
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || !is_wav_path(path)
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        anyhow::bail!("ループキャッシュに不正な相対パスがあります: {relative}");
    }
    Ok(())
}

fn save_index(index: &LoopIndex) -> Result<()> {
    let path = loop_index_path()?;
    let parent = path
        .parent()
        .ok_or_else(|| anyhow::anyhow!("ループキャッシュの親ディレクトリがありません"))?;
    std::fs::create_dir_all(parent).with_context(|| {
        format!(
            "ループキャッシュディレクトリを作成できません: {}",
            parent.display()
        )
    })?;
    let json = serde_json::to_vec_pretty(index)?;
    let temp_path = unique_temp_path(&path);
    let write_result = std::fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&temp_path)
        .and_then(|mut file| {
            use std::io::Write as _;
            file.write_all(&json)?;
            file.sync_all()
        });
    if let Err(error) = write_result {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error)
            .with_context(|| format!("一時ループキャッシュを書けません: {}", temp_path.display()));
    }
    if let Err(error) = replace_file(&temp_path, &path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error)
            .with_context(|| format!("ループキャッシュを置換できません: {}", path.display()));
    }
    Ok(())
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
mod tests;
