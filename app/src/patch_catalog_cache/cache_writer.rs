//! catalog の一時 file 作成と atomic replacement。

use super::CacheFile;
use anyhow::{Context, Result};
use std::{fs, path::Path};

pub(super) fn write_cache(path: &Path, cache: &CacheFile) -> Result<()> {
    let parent = path
        .parent()
        .context("patch catalog cacheの親directoryがありません")?;
    fs::create_dir_all(parent)?;
    let bytes = serde_json::to_vec_pretty(cache)?;
    let temp_path = path.with_extension(format!("json.{}.tmp", std::process::id()));
    fs::write(&temp_path, bytes)
        .with_context(|| format!("一時cacheを書けません: {}", temp_path.display()))?;
    if let Err(error) = replace_file(&temp_path, path) {
        let _ = fs::remove_file(&temp_path);
        return Err(error).with_context(|| format!("cacheを置換できません: {}", path.display()));
    }
    Ok(())
}

#[cfg(not(windows))]
pub(super) fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::rename(from, to)
}

#[cfg(windows)]
pub(super) fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let from: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
    let to: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
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
