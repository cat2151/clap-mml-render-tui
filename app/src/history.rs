//! history.json によるセッション状態の保存・復元。
//!
//! voicevox-playground-tui に倣い、終了時に現在行番号と編集行を保存し、
//! 起動時に復元する。

use std::{
    collections::hash_map::DefaultHasher,
    collections::HashMap,
    hash::{Hash, Hasher},
    path::PathBuf,
};

use anyhow::Result;
use serde_json::{Map, Value};

fn default_lines() -> Vec<String> {
    vec!["cde".to_string()]
}

/// 起動・終了で保存・復元するセッション状態。
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SessionState {
    /// 現在行番号（0始まり）。
    #[serde(default)]
    pub cursor: usize,
    /// 編集行リスト。
    #[serde(default = "default_lines")]
    pub lines: Vec<String>,
    /// 終了時に DAW モードだったかどうか。起動時に復元する。
    #[serde(default)]
    pub is_daw_mode: bool,
}

/// DAW 専用の履歴情報。
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DawSessionState {
    /// DAW カーソルの track 位置。
    #[serde(default)]
    pub cursor_track: usize,
    /// DAW カーソルの measure 位置。
    #[serde(default)]
    pub cursor_measure: usize,
    /// 既存 WAV キャッシュと対応付けるためのレンダリング用 MML ハッシュ。
    #[serde(default)]
    pub cached_measures: Vec<DawCachedMeasure>,
}

/// 既存 WAV キャッシュと一致確認するための track / measure ごとの MML ハッシュ。
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct DawCachedMeasure {
    pub track: usize,
    pub measure: usize,
    #[serde(default)]
    pub mml_hash: u64,
    #[serde(default, skip_serializing, alias = "mml")]
    pub(crate) legacy_mml: Option<String>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PatchPhraseStore {
    #[serde(default)]
    pub patches: HashMap<String, PatchPhraseState>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct PatchPhraseState {
    #[serde(default)]
    pub history: Vec<String>,
    #[serde(default)]
    pub favorites: Vec<String>,
}

impl DawCachedMeasure {
    fn normalize(&mut self) {
        if self.mml_hash == 0 {
            if let Some(mml) = self.legacy_mml.as_deref() {
                self.mml_hash = daw_cache_mml_hash(mml);
            }
        }
        self.legacy_mml = None;
    }
}

pub(crate) fn daw_cache_mml_hash(mml: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    mml.hash(&mut hasher);
    hasher.finish()
}

impl Default for SessionState {
    fn default() -> Self {
        Self {
            cursor: 0,
            lines: default_lines(),
            is_daw_mode: false,
        }
    }
}

/// OS ごとのデータディレクトリ配下の `clap-mml-render-tui` サブディレクトリを返す。
/// config.toml と同じ `clap-mml-render-tui` プレフィックスに揃えることで、ユーザーデータの場所を一貫させる。
/// `dirs::data_local_dir()` が利用できない環境では `None` を返し、保存・復元をスキップする。
fn history_dir() -> Option<PathBuf> {
    dirs::data_local_dir().map(|d| d.join("clap-mml-render-tui"))
}

fn session_state_path() -> Option<PathBuf> {
    history_dir().map(|d| d.join("history.json"))
}

fn daw_session_state_path() -> Option<PathBuf> {
    history_dir().map(|d| d.join("history_daw.json"))
}

fn patch_phrase_store_path() -> Option<PathBuf> {
    history_dir().map(|d| d.join("patch_history.json"))
}

/// DAW データファイル (`daw.json`) のパスを返す。
/// `history.json` と同じディレクトリに配置することでユーザーデータの場所を統一する。
/// `dirs::data_local_dir()` が利用できない環境では `None` を返す。
pub fn daw_file_path() -> Option<PathBuf> {
    history_dir().map(|d| d.join("daw.json"))
}

fn extract_daw_session_state(raw: &mut Map<String, Value>) -> Option<DawSessionState> {
    if let Some(daw) = raw.remove("daw") {
        return serde_json::from_value(daw).ok();
    }

    let mut daw = Map::new();
    let mut found = false;
    for (src, dst) in [
        ("daw_cursor_track", "cursor_track"),
        ("daw_cursor_measure", "cursor_measure"),
        ("daw_cached_measures", "cached_measures"),
    ] {
        if let Some(value) = raw.remove(src) {
            daw.insert(dst.to_string(), value);
            found = true;
        }
    }

    found
        .then(|| serde_json::from_value(Value::Object(daw)).ok())
        .flatten()
}

fn migrate_daw_session_state_from_history_json() -> Option<DawSessionState> {
    let path = session_state_path()?;
    if !path.exists() {
        return None;
    }

    let content = std::fs::read_to_string(&path).ok()?;
    let mut value = serde_json::from_str::<Value>(&content).ok()?;
    let raw = value.as_object_mut()?;
    let mut daw_state = extract_daw_session_state(raw)?;
    for measure in &mut daw_state.cached_measures {
        measure.normalize();
    }

    save_daw_session_state(&daw_state).ok()?;

    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).ok()?;
    }
    let rewritten = serde_json::to_string_pretty(&value).ok()?;
    std::fs::write(&path, rewritten).ok()?;
    Some(daw_state)
}

/// セッション状態（現在行番号）を history.json に保存する。
/// データディレクトリが利用できない場合はベストエフォートでスキップする。
pub fn save_session_state(state: &SessionState) -> Result<()> {
    let Some(path) = session_state_path() else {
        return Ok(());
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// DAW 専用履歴を history_daw.json に保存する。
pub fn save_daw_session_state(state: &DawSessionState) -> Result<()> {
    let Some(path) = daw_session_state_path() else {
        return Ok(());
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// patch ごとの phrase history / favorites を patch_history.json に保存する。
pub fn save_patch_phrase_store(store: &PatchPhraseStore) -> Result<()> {
    let Some(path) = patch_phrase_store_path() else {
        return Ok(());
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(store)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// history.json からセッション状態を読み込む。
/// ファイルが存在しない場合・データディレクトリが利用できない場合・読み込みに失敗した場合は
/// デフォルト値を返す。
/// `lines` が空の場合（`"lines": []` のような入力）はデフォルト値で補填し、
/// `lines` が常に1行以上という不変条件を保証する。
pub fn load_session_state() -> SessionState {
    let Some(path) = session_state_path() else {
        return SessionState::default();
    };
    if !path.exists() {
        return SessionState::default();
    }
    let mut state: SessionState = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default();
    if state.lines.is_empty() {
        state.lines = default_lines();
    }
    state
}

/// DAW 専用履歴を history_daw.json から読み込む。
/// ファイルが存在しない場合は、旧 history.json に埋め込まれていた DAW 情報を
/// 見つけたときのみ移行して返す。
pub fn load_daw_session_state() -> DawSessionState {
    let Some(path) = daw_session_state_path() else {
        return DawSessionState::default();
    };
    if path.exists() {
        let mut state: DawSessionState = std::fs::read_to_string(&path)
            .ok()
            .and_then(|s| serde_json::from_str(&s).ok())
            .unwrap_or_default();
        for measure in &mut state.cached_measures {
            measure.normalize();
        }
        return state;
    }
    migrate_daw_session_state_from_history_json().unwrap_or_default()
}

/// patch ごとの phrase history / favorites を patch_history.json から読み込む。
pub fn load_patch_phrase_store() -> PatchPhraseStore {
    let Some(path) = patch_phrase_store_path() else {
        return PatchPhraseStore::default();
    };
    if !path.exists() {
        return PatchPhraseStore::default();
    }
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

#[cfg(test)]
#[path = "history_tests.rs"]
mod tests;
