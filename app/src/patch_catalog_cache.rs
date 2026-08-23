//! 明示的CLIで構築し、TUIからは読み取り専用で使うpatch catalog cache。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cmrt_runtime::{CatalogPlugin, Config, PatchRoles, SkippedCatalogPlugin};
use cmrt_tui_core::patch_load::PatchCatalogSnapshot;
use serde::{Deserialize, Serialize};

const CACHE_FORMAT_VERSION: u32 = 2;
const CACHE_RELATIVE_PATH: &str = "patch-catalog/catalog.json";
pub const BUILD_COMMAND: &str = "cmrt build-patch-catalog-cache";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildSummary {
    pub path: PathBuf,
    pub patch_count: usize,
    pub plugin_names: Vec<String>,
    pub vvp_voicing_count: usize,
    pub vvp_unknown_count: usize,
}

pub struct LoadedPatchCatalogCache {
    snapshot: PatchCatalogSnapshot,
    vvp_voicings: BTreeMap<String, cmrt_realtime_play::PatchVoicing>,
}

impl LoadedPatchCatalogCache {
    pub fn into_parts(
        self,
    ) -> (
        PatchCatalogSnapshot,
        BTreeMap<String, cmrt_realtime_play::PatchVoicing>,
    ) {
        (self.snapshot, self.vvp_voicings)
    }
}

#[derive(Serialize, Deserialize)]
struct CacheFile {
    format_version: u32,
    patches: Vec<String>,
    plugins: Vec<CachedPlugin>,
    #[serde(default)]
    vvp_voicings: BTreeMap<String, cmrt_realtime_play::PatchVoicing>,
    #[serde(default)]
    catalog_notes: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct CachedPlugin {
    name: String,
    plugin_path: String,
    plugin_id: Option<String>,
    base: Option<String>,
    dirs: Vec<String>,
    #[serde(default)]
    source_notices: Vec<String>,
    patch_roles: PatchRoles,
}

impl From<&CatalogPlugin> for CachedPlugin {
    fn from(plugin: &CatalogPlugin) -> Self {
        Self {
            name: plugin.name.clone(),
            plugin_path: plugin.plugin_path.clone(),
            plugin_id: plugin.plugin_id.clone(),
            base: plugin.base.clone(),
            dirs: plugin.dirs.clone(),
            source_notices: plugin.source_notices.clone(),
            patch_roles: plugin.patch_roles.clone(),
        }
    }
}

impl From<CachedPlugin> for CatalogPlugin {
    fn from(plugin: CachedPlugin) -> Self {
        Self {
            name: plugin.name,
            plugin_path: plugin.plugin_path,
            plugin_id: plugin.plugin_id,
            base: plugin.base,
            dirs: plugin.dirs,
            resolved_patches: None,
            source_notices: plugin.source_notices,
            patch_roles: plugin.patch_roles,
        }
    }
}

pub fn cache_file_path() -> Option<PathBuf> {
    crate::config::config_app_dir().map(|dir| dir.join(CACHE_RELATIVE_PATH))
}

pub fn build_and_save(cfg: &Config) -> Result<BuildSummary> {
    if cfg.plugin_path.trim().is_empty() {
        anyhow::bail!("plugin_pathが空のためpatch catalog cacheを構築できません");
    }
    let path = cache_file_path().context("patch catalog cacheの保存先を取得できません")?;
    let (plugins, skipped) = cmrt_runtime::catalog_plugins_detailed(cfg);
    let pairs = cmrt_tui_core::patches::collect_patch_pairs_from_catalog(&plugins)?;
    let vvp_voicings = collect_vvp_voicings(&plugins, &pairs);
    let vvp_unknown_count = vvp_voicings
        .values()
        .filter(|voicing| **voicing == cmrt_realtime_play::PatchVoicing::Unknown)
        .count();
    let catalog_notes = catalog_notes(&plugins, &skipped);
    let cache = CacheFile {
        format_version: CACHE_FORMAT_VERSION,
        patches: pairs.into_iter().map(|(display, _)| display).collect(),
        plugins: plugins.iter().map(CachedPlugin::from).collect(),
        vvp_voicings,
        catalog_notes,
    };
    write_cache(&path, &cache)?;
    Ok(BuildSummary {
        path,
        patch_count: cache.patches.len(),
        plugin_names: plugins.into_iter().map(|plugin| plugin.name).collect(),
        vvp_voicing_count: cache.vvp_voicings.len(),
        vvp_unknown_count,
    })
}

pub fn load() -> Result<LoadedPatchCatalogCache> {
    let path = cache_file_path().context("patch catalog cacheの保存先を取得できません")?;
    load_from(&path)
}

fn load_from(path: &Path) -> Result<LoadedPatchCatalogCache> {
    let bytes = fs::read(path)
        .with_context(|| format!("patch catalog cacheを読めません: {}", path.display()))?;
    let cache: CacheFile = serde_json::from_slice(&bytes)
        .with_context(|| format!("patch catalog cacheが不正です: {}", path.display()))?;
    if cache.format_version != CACHE_FORMAT_VERSION {
        anyhow::bail!(
            "patch catalog cacheのformat versionが非対応です: expected={}, actual={}",
            CACHE_FORMAT_VERSION,
            cache.format_version
        );
    }
    if cache.plugins.is_empty() {
        anyhow::bail!("patch catalog cacheにpluginがありません");
    }
    if cache
        .plugins
        .iter()
        .any(|plugin| plugin.plugin_path.trim().is_empty())
    {
        anyhow::bail!("patch catalog cacheにplugin_pathが空のpluginがあります");
    }
    if let Some(display) = cache
        .patches
        .iter()
        .filter(|display| cmrt_core::is_vvp_patch_path(display))
        .find(|display| !cache.vvp_voicings.contains_key(*display))
    {
        anyhow::bail!("patch catalog cacheにVVP voicingがありません: {display}");
    }
    let vvp_voicings = cache.vvp_voicings;
    let pairs = cache
        .patches
        .into_iter()
        .map(|display| {
            let lower = display.to_lowercase();
            (display, lower)
        })
        .collect();
    Ok(LoadedPatchCatalogCache {
        snapshot: PatchCatalogSnapshot::new(
            pairs,
            cache.plugins.into_iter().map(CatalogPlugin::from).collect(),
            cache.catalog_notes,
        ),
        vvp_voicings,
    })
}

fn collect_vvp_voicings(
    plugins: &[CatalogPlugin],
    pairs: &[(String, String)],
) -> BTreeMap<String, cmrt_realtime_play::PatchVoicing> {
    let patch_plugins = cmrt_tui_core::patch_plugins::PatchPlugins::from_catalog(plugins.to_vec());
    pairs
        .iter()
        .filter(|(display, _)| cmrt_core::is_vvp_patch_path(display))
        .map(|(display, _)| {
            let plugin = patch_plugins.for_patch(display);
            let path = match plugin.base.as_deref() {
                Some(base) => Path::new(base).join(display),
                None => PathBuf::from(display),
            };
            let voicing = match cmrt_core::read_vvp_header(&path) {
                Ok(header) if header.poly => cmrt_realtime_play::PatchVoicing::Poly,
                Ok(_) => cmrt_realtime_play::PatchVoicing::Mono,
                Err(_) => cmrt_realtime_play::PatchVoicing::Unknown,
            };
            (display.clone(), voicing)
        })
        .collect()
}

fn catalog_notes(plugins: &[CatalogPlugin], skipped: &[SkippedCatalogPlugin]) -> Vec<String> {
    plugins
        .iter()
        .flat_map(|plugin| {
            plugin
                .source_notices
                .iter()
                .map(move |notice| format!("{}: {notice}", plugin.name))
        })
        .chain(skipped.iter().map(SkippedCatalogPlugin::notice_line))
        .collect()
}

fn write_cache(path: &Path, cache: &CacheFile) -> Result<()> {
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
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
    fs::rename(from, to)
}

#[cfg(windows)]
fn replace_file(from: &Path, to: &Path) -> std::io::Result<()> {
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

#[cfg(test)]
mod tests;
