//! play server が起動時に読む、音源catalogの軽量な永続cache。
//!
//! 完全なpatch一覧とload計測は親moduleの`catalog.json`に残す。serverが起動時に要るのは
//! pluginごとのrootだけなので、同じ明示CLI scanの結果から小さな別fileを併記する。
//! 対応するreaderはplay-serverの`core-lib/src/plugin_catalog/source_cache.rs`。

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cmrt_runtime::CatalogPlugin;
use serde::Serialize;

const FORMAT_VERSION: u32 = 1;
const CACHE_RELATIVE_PATH: &str = "patch-catalog/sources.json";

#[derive(Serialize)]
struct SourceCache<'a> {
    format_version: u32,
    plugins: Vec<CachedSource<'a>>,
}

#[derive(Serialize)]
struct CachedSource<'a> {
    name: &'a str,
    plugin_path: &'a str,
    plugin_id: Option<&'a str>,
    base: Option<&'a str>,
    dirs: &'a [String],
    source_notices: &'a [String],
}

pub(super) fn cache_file_path() -> Option<PathBuf> {
    crate::config::config_app_dir().map(|dir| dir.join(CACHE_RELATIVE_PATH))
}

pub(super) fn write(path: &Path, plugins: &[CatalogPlugin]) -> Result<()> {
    let cache = SourceCache {
        format_version: FORMAT_VERSION,
        plugins: plugins
            .iter()
            .map(|plugin| CachedSource {
                name: &plugin.name,
                plugin_path: &plugin.plugin_path,
                plugin_id: plugin.plugin_id.as_deref(),
                base: plugin.base.as_deref(),
                dirs: &plugin.dirs,
                source_notices: &plugin.source_notices,
            })
            .collect(),
    };
    let parent = path
        .parent()
        .context("catalog source cacheの親directoryがありません")?;
    std::fs::create_dir_all(parent)?;
    let bytes = serde_json::to_vec_pretty(&cache)?;
    let temp_path = path.with_extension(format!("json.{}.tmp", std::process::id()));
    std::fs::write(&temp_path, bytes)
        .with_context(|| format!("一時cacheを書けません: {}", temp_path.display()))?;
    if let Err(error) = super::replace_file(&temp_path, path) {
        let _ = std::fs::remove_file(&temp_path);
        return Err(error).with_context(|| format!("cacheを置換できません: {}", path.display()));
    }
    Ok(())
}

#[cfg(test)]
mod tests;
