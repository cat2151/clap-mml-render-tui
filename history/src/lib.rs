//! history.json によるセッション状態の保存・復元。
//!
//! voicevox-playground-tui に倣い、終了時に現在行番号と編集行を保存し、
//! 起動時に復元する。notepad / DAW / keyboard の各画面から共有されるため、
//! app からは独立した crate として切り出してある。
//!
//! テスト時のディレクトリ差し替えは `test-support` feature で有効になる
//! [`test_support`] モジュールが担う。prod ビルドでは一切コンパイルされない。

use std::path::PathBuf;

mod daw;
mod helpers;
mod patch_phrase_store;
mod paths;
mod session_state;
mod voicing_cache;

#[cfg(any(test, feature = "test-support"))]
pub mod test_support;

pub use cmrt_tui_core::screen_switch::PrimaryScreen;
pub use daw::{
    daw_cache_mml_hash, load_daw_session_state, save_daw_session_state,
    save_daw_sound_check_guide_overlay_date, DawCachedMeasure, DawSessionState,
};
pub use patch_phrase_store::{
    load_patch_phrase_store, normalize_patch_phrase_store_for_available_patches,
    rename_patch_phrase_store_key, save_patch_phrase_store, sync_patch_favorite_order,
    touch_patch_favorite, PatchPhraseState, PatchPhraseStore,
};
pub use paths::{daw_file_load_path, daw_file_path};
pub use session_state::{
    load_session_state, save_keyboard_note_guide_overlay_date,
    save_notepad_sound_check_guide_overlay_date, save_session_state, KeyboardSessionState,
    KeyboardTransport, SessionState,
};
pub use voicing_cache::{load_voicing_cache, save_voicing_cache, VoicingCache};

#[cfg(test)]
use paths::{
    daw_session_state_path, history_dir, patch_phrase_store_path, session_state_path,
    voicing_cache_path,
};

const APP_DIR_NAME: &str = "clap-mml-render-tui";
const HISTORY_DIR_NAME: &str = "history";

/// history ファイルの配置先 app ディレクトリ。
///
/// prod では常に `None`（OS 標準ディレクトリへフォールバック）。
/// テスト時のみ [`test_support`] が差し替える。
#[cfg(any(test, feature = "test-support"))]
fn test_history_app_dir() -> Option<PathBuf> {
    test_support::current_thread_app_dir().or_else(test_support::default_test_app_dir)
}

#[cfg(not(any(test, feature = "test-support")))]
fn test_history_app_dir() -> Option<PathBuf> {
    None
}

#[cfg(test)]
mod tests;
