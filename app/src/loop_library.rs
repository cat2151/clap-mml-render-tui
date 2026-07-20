//! WAV ループライブラリの走査と永続インデックス。

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::ffi::OsStr;
use std::path::{Component, Path, PathBuf};

const LOOP_INDEX_VERSION: u32 = 1;
const LOOP_INDEX_FILE_NAME: &str = "loop_index.json";

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub(crate) struct LoopIndex {
    pub(crate) version: u32,
    pub(crate) roots: Vec<LoopRootIndex>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Eq, Serialize)]
pub(crate) struct LoopRootIndex {
    pub(crate) path: String,
    pub(crate) wav_files: Vec<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LoopScanSummary {
    pub roots: usize,
    pub wav_files: usize,
}

pub fn scan_and_save(cfg: &crate::config::Config) -> Result<LoopScanSummary> {
    scan_dirs_and_save(&cfg.loop_dirs)
}

fn scan_dirs_and_save(loop_dirs: &[String]) -> Result<LoopScanSummary> {
    let index = build_index(loop_dirs)?;
    let summary = LoopScanSummary {
        roots: index.roots.len(),
        wav_files: index.roots.iter().map(|root| root.wav_files.len()).sum(),
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

fn build_index(loop_dirs: &[String]) -> Result<LoopIndex> {
    if loop_dirs.is_empty() {
        anyhow::bail!("loop_dirs が空です。config.toml にルートを設定してください");
    }

    let mut roots = Vec::with_capacity(loop_dirs.len());
    for configured_root in loop_dirs {
        let root = Path::new(configured_root);
        if !root.is_dir() {
            anyhow::bail!(
                "loop_dirs のパスがディレクトリではありません: {}",
                root.display()
            );
        }
        let mut wav_paths = Vec::new();
        collect_wav_paths(root, root, &mut wav_paths)?;
        wav_paths.sort_by(|left, right| {
            left.to_lowercase()
                .cmp(&right.to_lowercase())
                .then_with(|| left.cmp(right))
        });
        roots.push(LoopRootIndex {
            path: configured_root.clone(),
            wav_files: wav_paths,
        });
    }

    Ok(LoopIndex {
        version: LOOP_INDEX_VERSION,
        roots,
    })
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
        for relative in &root.wav_files {
            validate_relative_wav_path(relative)?;
        }
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
mod tests {
    use super::*;

    fn create_file(path: &Path) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"test").unwrap();
    }

    #[test]
    fn build_index_collects_only_wav_files_and_sorts_them() {
        let temp = tempfile_dir("collect");
        create_file(&temp.join("Pack/Bass/z.WAV"));
        create_file(&temp.join("Pack/Bass/A.wav"));
        create_file(&temp.join("Pack/Bass/readme.txt"));

        let index = build_index(&[temp.to_string_lossy().into_owned()]).unwrap();

        assert_eq!(index.roots.len(), 1);
        assert_eq!(
            index.roots[0].wav_files,
            [
                PathBuf::from("Pack")
                    .join("Bass")
                    .join("A.wav")
                    .to_string_lossy(),
                PathBuf::from("Pack")
                    .join("Bass")
                    .join("z.WAV")
                    .to_string_lossy(),
            ]
        );
    }

    #[test]
    fn scan_replaces_cache_and_failure_preserves_previous_cache() {
        let temp = tempfile_dir("save");
        let _guard = crate::test_utils::set_local_dir_envs(&temp);
        let first_root = temp.join("first");
        create_file(&first_root.join("One.wav"));
        let first_dirs = vec![first_root.to_string_lossy().into_owned()];
        let first_summary = scan_dirs_and_save(&first_dirs).unwrap();
        assert_eq!(first_summary.wav_files, 1);
        let cache_path = loop_index_path().unwrap();
        let first_cache = std::fs::read(&cache_path).unwrap();

        create_file(&first_root.join("Two.wav"));
        let second_summary = scan_dirs_and_save(&first_dirs).unwrap();
        assert_eq!(second_summary.wav_files, 2);
        let second_cache = std::fs::read(&cache_path).unwrap();
        assert_ne!(first_cache, second_cache);

        let missing_dirs = vec![temp.join("missing").to_string_lossy().into_owned()];
        assert!(scan_dirs_and_save(&missing_dirs).is_err());
        assert_eq!(std::fs::read(cache_path).unwrap(), second_cache);
    }

    #[test]
    fn validate_index_rejects_version_roots_and_parent_paths() {
        let valid = LoopIndex {
            version: LOOP_INDEX_VERSION,
            roots: vec![LoopRootIndex {
                path: "/loops".to_string(),
                wav_files: vec!["pack/kick.wav".to_string()],
            }],
        };
        validate_index(&valid, &["/loops".to_string()]).unwrap();

        let mut wrong_version = valid.clone();
        wrong_version.version += 1;
        assert!(validate_index(&wrong_version, &["/loops".to_string()]).is_err());
        assert!(validate_index(&valid, &["/other".to_string()]).is_err());

        let mut unsafe_path = valid;
        unsafe_path.roots[0].wav_files = vec!["../outside.wav".to_string()];
        assert!(validate_index(&unsafe_path, &["/loops".to_string()]).is_err());
    }

    fn tempfile_dir(label: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!(
            "cmrt_loop_library_{label}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&path).unwrap();
        path
    }
}
