use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

use anyhow::Result;
use serde_json::{Map, Value};

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
    let path = super::paths::resolved_history_file_path("history.json")?;
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

/// DAW 専用履歴を history_daw.json に保存する。
pub fn save_daw_session_state(state: &DawSessionState) -> Result<()> {
    let _ = super::paths::migrate_legacy_history_file("history_daw.json");
    let Some(path) = super::paths::daw_session_state_path() else {
        return Ok(());
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(&path, json)?;
    Ok(())
}

/// DAW 専用履歴を history_daw.json から読み込む。
/// ファイルが存在しない場合は、旧 history.json に埋め込まれていた DAW 情報を
/// 見つけたときのみ移行して返す。
pub fn load_daw_session_state() -> DawSessionState {
    let Some(path) = super::paths::resolved_history_file_path("history_daw.json") else {
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
