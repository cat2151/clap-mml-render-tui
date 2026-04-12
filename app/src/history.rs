//! history.json によるセッション状態の保存・復元。
//!
//! voicevox-playground-tui に倣い、終了時に現在行番号と編集行を保存し、
//! 起動時に復元する。

#[cfg(test)]
use std::cell::RefCell;
use std::path::PathBuf;

mod daw;
mod helpers;
mod patch_phrase_store;
mod paths;
mod session_state;

pub(crate) use daw::daw_cache_mml_hash;
pub use daw::{load_daw_session_state, save_daw_session_state, DawCachedMeasure, DawSessionState};
pub use patch_phrase_store::{
    load_patch_phrase_store, save_patch_phrase_store, PatchPhraseState, PatchPhraseStore,
};
pub(crate) use patch_phrase_store::{
    normalize_patch_phrase_store_for_available_patches, rename_patch_phrase_store_key,
    sync_patch_favorite_order, touch_patch_favorite,
};
pub(crate) use paths::daw_file_load_path;
pub use paths::daw_file_path;
pub use session_state::{load_session_state, save_session_state, SessionState};

#[cfg(test)]
use paths::{daw_session_state_path, history_dir, patch_phrase_store_path, session_state_path};

const APP_DIR_NAME: &str = "clap-mml-render-tui";
const HISTORY_DIR_NAME: &str = "history";

#[cfg(test)]
thread_local! {
    static TEST_HISTORY_APP_DIR: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
}

#[cfg(test)]
fn test_history_app_dir() -> Option<PathBuf> {
    TEST_HISTORY_APP_DIR
        .with(|dir| dir.borrow().clone())
        .or_else(crate::test_utils::default_test_app_dir)
}

#[cfg(not(test))]
fn test_history_app_dir() -> Option<PathBuf> {
    None
}

#[cfg(test)]
pub(crate) fn set_test_history_app_dir_for_current_thread(
    path: Option<PathBuf>,
) -> Option<PathBuf> {
    TEST_HISTORY_APP_DIR.with(|dir| dir.replace(path))
}

#[cfg(test)]
pub(crate) fn test_history_app_dir_for_current_thread() -> Option<PathBuf> {
    TEST_HISTORY_APP_DIR.with(|dir| dir.borrow().clone())
}

#[cfg(not(test))]
#[allow(dead_code)]
pub(crate) fn set_test_history_app_dir_for_current_thread(_: Option<PathBuf>) -> Option<PathBuf> {
    None
}

#[cfg(not(test))]
#[allow(dead_code)]
pub(crate) fn test_history_app_dir_for_current_thread() -> Option<PathBuf> {
    None
}

#[cfg(test)]
#[path = "tests/history.rs"]
mod tests;
