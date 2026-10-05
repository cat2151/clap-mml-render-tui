use std::collections::HashSet;
use std::io::ErrorKind;
use std::path::Path;

use anyhow::{Context, Result};
use serde::de::DeserializeOwned;

pub(crate) fn read_json_or_default<T: DeserializeOwned + Default>(path: &Path) -> Result<T> {
    let json = match std::fs::read_to_string(path) {
        Ok(json) => json,
        Err(error) if error.kind() == ErrorKind::NotFound => return Ok(T::default()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("保存データを読み込めません: {}", path.display()));
        }
    };
    serde_json::from_str(&json)
        .with_context(|| format!("保存データを解析できません: {}", path.display()))
}

pub(crate) fn write_json<T: serde::Serialize>(path: &Path, state: &T) -> Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)
            .with_context(|| format!("保存先を作成できません: {}", dir.display()))?;
    }
    let json = serde_json::to_string_pretty(state)?;
    std::fs::write(path, json)
        .with_context(|| format!("保存データを書き込めません: {}", path.display()))
}

pub(crate) fn default_lines() -> Vec<String> {
    vec!["cde".to_string()]
}

pub(crate) fn merge_patch_phrase_items(dest: &mut Vec<String>, src: Vec<String>) {
    let mut seen = dest.iter().cloned().collect::<HashSet<_>>();
    for item in src {
        if seen.insert(item.clone()) {
            dest.push(item);
        }
    }
}
