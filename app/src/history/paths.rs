use std::path::PathBuf;

use super::{APP_DIR_NAME, HISTORY_DIR_NAME};

pub(super) fn history_dir() -> Option<PathBuf> {
    super::test_history_app_dir()
        .map(|d| d.join(HISTORY_DIR_NAME))
        .or_else(|| dirs::config_local_dir().map(|d| d.join(APP_DIR_NAME).join(HISTORY_DIR_NAME)))
}

fn legacy_history_dir() -> Option<PathBuf> {
    super::test_history_app_dir().or_else(|| dirs::data_local_dir().map(|d| d.join(APP_DIR_NAME)))
}

fn history_file_path(file_name: &str) -> Option<PathBuf> {
    history_dir().map(|d| d.join(file_name))
}

fn legacy_history_file_path(file_name: &str) -> Option<PathBuf> {
    legacy_history_dir().map(|d| d.join(file_name))
}

pub(super) fn migrate_legacy_history_file(file_name: &str) -> Option<PathBuf> {
    let path = history_file_path(file_name)?;
    if path.exists() {
        return Some(path);
    }
    let legacy_path = legacy_history_file_path(file_name)?;
    if !legacy_path.exists() {
        return Some(path);
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok()?;
    }
    if std::fs::rename(&legacy_path, &path).is_err() {
        std::fs::copy(&legacy_path, &path).ok()?;
        std::fs::remove_file(&legacy_path).ok();
    }
    Some(path)
}

pub(super) fn resolved_history_file_path(file_name: &str) -> Option<PathBuf> {
    let path = history_file_path(file_name)?;
    if path.exists() {
        return Some(path);
    }
    let legacy_path = legacy_history_file_path(file_name)?;
    if !legacy_path.exists() {
        return Some(path);
    }
    match migrate_legacy_history_file(file_name) {
        Some(migrated) if migrated.exists() => Some(migrated),
        _ => Some(legacy_path),
    }
}

pub(super) fn session_state_path() -> Option<PathBuf> {
    history_file_path("history.json")
}

pub(super) fn daw_session_state_path() -> Option<PathBuf> {
    history_file_path("history_daw.json")
}

pub(super) fn patch_phrase_store_path() -> Option<PathBuf> {
    history_file_path("patch_history.json")
}

/// DAW データファイル (`daw.json`) のパスを返す。
/// `history.json` と同じディレクトリに配置することでユーザーデータの場所を統一する。
/// `dirs::config_local_dir()` が利用できない環境では `None` を返す。
pub fn daw_file_path() -> Option<PathBuf> {
    history_file_path("daw.json")
}

pub(crate) fn daw_file_load_path() -> Option<PathBuf> {
    resolved_history_file_path("daw.json")
}
