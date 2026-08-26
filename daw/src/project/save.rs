use std::{
    ffi::OsString,
    io::ErrorKind,
    path::{Path, PathBuf},
};

use anyhow::{bail, Context, Result};

fn backup_candidate(path: &Path, index: usize) -> PathBuf {
    let mut candidate: OsString = path.as_os_str().to_owned();
    if index == 0 {
        candidate.push(".bak");
    } else {
        candidate.push(format!(".bak.{index}"));
    }
    PathBuf::from(candidate)
}

fn available_backup_path(path: &Path) -> Result<PathBuf> {
    for index in 0.. {
        let candidate = backup_candidate(path, index);
        match std::fs::symlink_metadata(&candidate) {
            Ok(_) => {}
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(candidate),
            Err(error) => {
                return Err(error).with_context(|| {
                    format!("backup path を確認できません: {}", candidate.display())
                });
            }
        }
    }
    unreachable!("backup suffix index is unbounded")
}

fn backup_existing_file(path: &Path) -> Result<Option<PathBuf>> {
    match std::fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_dir() => {
            bail!("project file path が directory です: {}", path.display())
        }
        Ok(_) => {
            let backup_path = available_backup_path(path)?;
            std::fs::rename(path, &backup_path).with_context(|| {
                format!(
                    "既存 project file を backup へ移動できません: {} -> {}",
                    path.display(),
                    backup_path.display()
                )
            })?;
            Ok(Some(backup_path))
        }
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(None),
        Err(error) => {
            Err(error).with_context(|| format!("project file を確認できません: {}", path.display()))
        }
    }
}

fn restore_backup(path: &Path, backup_path: &Path, write_error: std::io::Error) -> anyhow::Error {
    if let Err(remove_error) = std::fs::remove_file(path) {
        if remove_error.kind() != ErrorKind::NotFound {
            return anyhow::anyhow!(
                "project file の保存に失敗し、旧 file の復旧前に不完全な file を削除できませんでした: {write_error}; path={}; backup={}; cleanup error={remove_error}",
                path.display(),
                backup_path.display()
            );
        }
    }

    match std::fs::rename(backup_path, path) {
        Ok(()) => anyhow::Error::new(write_error).context(format!(
            "project file を保存できません（旧 file は復旧済み）: {}",
            path.display()
        )),
        Err(restore_error) => anyhow::anyhow!(
            "project file の保存と旧 file の復旧に失敗しました: {write_error}; path={}; backup={}; restore error={restore_error}",
            path.display(),
            backup_path.display()
        ),
    }
}

pub(super) fn write_project_file(path: &Path, contents: &[u8]) -> Result<Option<PathBuf>> {
    let backup_path = backup_existing_file(path)?;
    match std::fs::write(path, contents) {
        Ok(()) => Ok(backup_path),
        Err(write_error) => match backup_path {
            Some(backup_path) => Err(restore_backup(path, &backup_path, write_error)),
            None => Err(write_error)
                .with_context(|| format!("project file を保存できません: {}", path.display())),
        },
    }
}
