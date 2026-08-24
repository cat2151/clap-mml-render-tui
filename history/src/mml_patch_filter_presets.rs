//! MML patch selector で追加した正規表現プリセットの永続化。

use std::collections::HashSet;

use anyhow::Result;

#[derive(Default, serde::Serialize, serde::Deserialize)]
struct PresetStore {
    #[serde(default)]
    presets: Vec<PresetEntry>,
}

#[derive(serde::Serialize, serde::Deserialize)]
struct PresetEntry {
    group: String,
    pattern: String,
}

pub fn load_mml_patch_filter_presets() -> Vec<(String, String)> {
    let Some(path) = super::paths::resolved_history_file_path("mml_patch_filters.json") else {
        return Vec::new();
    };
    if !path.exists() {
        return Vec::new();
    }
    let store = std::fs::read_to_string(path)
        .ok()
        .and_then(|json| serde_json::from_str::<PresetStore>(&json).ok())
        .unwrap_or_default();
    normalize(
        store
            .presets
            .into_iter()
            .map(|entry| (entry.group, entry.pattern))
            .collect(),
    )
}

pub fn save_mml_patch_filter_presets(presets: &[(String, String)]) -> Result<()> {
    let _ = super::paths::migrate_legacy_history_file("mml_patch_filters.json");
    let Some(path) = super::paths::mml_patch_filter_presets_path() else {
        return Ok(());
    };
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let store = PresetStore {
        presets: normalize(presets.to_vec())
            .into_iter()
            .map(|(group, pattern)| PresetEntry { group, pattern })
            .collect(),
    };
    std::fs::write(path, serde_json::to_string_pretty(&store)?)?;
    Ok(())
}

fn normalize(presets: Vec<(String, String)>) -> Vec<(String, String)> {
    let mut seen = HashSet::new();
    presets
        .into_iter()
        .filter_map(|(group, pattern)| {
            let group = group.trim();
            let pattern = pattern.trim();
            let entry = (group.to_string(), pattern.to_string());
            (!group.is_empty() && !pattern.is_empty() && seen.insert(entry.clone()))
                .then_some(entry)
        })
        .collect()
}

#[cfg(test)]
mod tests;
