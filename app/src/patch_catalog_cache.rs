//! 明示的CLIで構築し、TUIからは読み取り専用で使うpatch catalog cache。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cmrt_runtime::{CatalogPlugin, Config, SkippedCatalogPlugin};
use cmrt_tui_core::patch_load::{PatchCatalogSnapshot, PatchLoadMeasurement};
use serde::{Deserialize, Serialize};

use measurements::collect_patch_load_measurements;
#[cfg(test)]
use measurements::{estimate_eta, format_eta, measure_patch_loads};

mod measurements;

const CACHE_FORMAT_VERSION: u32 = 4;
const CACHE_RELATIVE_PATH: &str = "patch-catalog/catalog.json";
pub const BUILD_COMMAND: &str = "cmrt build-patch-catalog-cache";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildSummary {
    pub path: PathBuf,
    pub patch_count: usize,
    pub plugin_names: Vec<String>,
    pub catalog_voicing_count: usize,
    pub catalog_unknown_count: usize,
    pub measured_load_count: usize,
    pub first_load_failure_count: usize,
    pub second_load_failure_count: usize,
}

pub struct LoadedPatchCatalogCache {
    snapshot: PatchCatalogSnapshot,
    patch_voicings: BTreeMap<String, cmrt_realtime_play::PatchVoicing>,
}

impl LoadedPatchCatalogCache {
    pub fn into_parts(
        self,
    ) -> (
        PatchCatalogSnapshot,
        BTreeMap<String, cmrt_realtime_play::PatchVoicing>,
    ) {
        (self.snapshot, self.patch_voicings)
    }
}

#[derive(Serialize, Deserialize)]
struct CacheFile {
    format_version: u32,
    patches: Vec<CachedPatch>,
    plugins: Vec<CachedPlugin>,
    #[serde(default)]
    patch_voicings: BTreeMap<String, cmrt_realtime_play::PatchVoicing>,
    #[serde(default)]
    catalog_notes: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct CachedPatch {
    /// server shared coreが解釈したplugin key・表示名・分類・voicing。
    audio: cmrt_core::AudioPatch,
    #[serde(flatten)]
    measurement: PatchLoadMeasurement,
}

#[derive(Clone, Serialize, Deserialize)]
struct CachedPlugin {
    name: String,
    plugin_path: String,
    plugin_id: Option<String>,
    base: Option<String>,
    dirs: Vec<String>,
    #[serde(default)]
    source_notices: Vec<String>,
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
    let audio_patches = describe_patches(&plugins, &pairs)?;
    let patch_voicings = collect_patch_voicings(&plugins, &audio_patches);
    let catalog_unknown_count = patch_voicings
        .values()
        .filter(|voicing| **voicing == cmrt_realtime_play::PatchVoicing::Unknown)
        .count();
    let load_measurements = collect_patch_load_measurements(cfg, &pairs)?;
    let measured_load_count = load_measurements
        .values()
        .filter(|measurement| measurement.second_load_ms.is_some())
        .count();
    let first_load_failure_count = load_measurements
        .values()
        .filter(|measurement| measurement.first_load_error.is_some())
        .count();
    let second_load_failure_count = load_measurements
        .values()
        .filter(|measurement| measurement.second_load_error.is_some())
        .count();
    let catalog_notes = catalog_notes(&plugins, &skipped);
    let cache = CacheFile {
        format_version: CACHE_FORMAT_VERSION,
        patches: audio_patches
            .into_iter()
            .map(|audio| {
                let display = &audio.reference.display;
                let measurement = load_measurements
                    .get(display)
                    .cloned()
                    .with_context(|| format!("patch load計測結果がありません: {display}"))?;
                Ok(CachedPatch { audio, measurement })
            })
            .collect::<Result<Vec<_>>>()?,
        plugins: plugins.iter().map(CachedPlugin::from).collect(),
        patch_voicings,
        catalog_notes,
    };
    write_cache(&path, &cache)?;
    Ok(BuildSummary {
        path,
        patch_count: cache.patches.len(),
        plugin_names: plugins.into_iter().map(|plugin| plugin.name).collect(),
        catalog_voicing_count: cache.patch_voicings.len(),
        catalog_unknown_count,
        measured_load_count,
        first_load_failure_count,
        second_load_failure_count,
    })
}

pub fn load() -> Result<LoadedPatchCatalogCache> {
    let path = cache_file_path().context("patch catalog cacheの保存先を取得できません")?;
    load_from(&path)
}

fn load_from(path: &Path) -> Result<LoadedPatchCatalogCache> {
    let bytes = fs::read(path)
        .with_context(|| format!("patch catalog cacheを読めません: {}", path.display()))?;
    let value: serde_json::Value = serde_json::from_slice(&bytes)
        .with_context(|| format!("patch catalog cacheが不正です: {}", path.display()))?;
    let format_version = value
        .get("format_version")
        .and_then(serde_json::Value::as_u64)
        .context("patch catalog cacheにformat_versionがありません")?;
    if format_version != u64::from(CACHE_FORMAT_VERSION) {
        anyhow::bail!(
            "patch catalog cacheのformat versionが非対応です: expected={}, actual={}",
            CACHE_FORMAT_VERSION,
            format_version
        );
    }
    let mut cache: CacheFile = serde_json::from_value(value)
        .with_context(|| format!("patch catalog cacheが不正です: {}", path.display()))?;
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
    validate_catalog_voicings(&cache)?;
    populate_selector_categories(&mut cache)?;
    if let Some(display) = cache
        .patches
        .iter()
        .find(|patch| {
            patch.measurement.second_load_ms.is_none()
                && patch.measurement.second_load_error.is_none()
        })
        .map(|patch| &patch.audio.reference.display)
    {
        anyhow::bail!("patch catalog cacheに2回目のload計測結果がありません: {display}");
    }
    let patch_voicings = cache.patch_voicings;
    let mut load_measurements = BTreeMap::new();
    let audio_patches = cache
        .patches
        .into_iter()
        .map(|patch| {
            let display = patch.audio.reference.display.clone();
            load_measurements.insert(display.clone(), patch.measurement);
            patch.audio
        })
        .collect::<Vec<_>>();
    let pairs = audio_patches
        .iter()
        .map(|patch| {
            (
                patch.reference.display.clone(),
                patch.normalized_display.clone(),
            )
        })
        .collect();
    Ok(LoadedPatchCatalogCache {
        snapshot: PatchCatalogSnapshot::new(
            pairs,
            audio_patches,
            cache.plugins.into_iter().map(CatalogPlugin::from).collect(),
            cache.catalog_notes,
            load_measurements,
        ),
        patch_voicings,
    })
}

/// optional field追加前のcatalogも、plugin固有I/Oや全patch再計測なしで補完する。
fn populate_selector_categories(cache: &mut CacheFile) -> Result<()> {
    let plugins = cache
        .plugins
        .iter()
        .cloned()
        .map(CatalogPlugin::from)
        .collect::<Vec<_>>();
    let patch_plugins = cmrt_tui_core::patch_plugins::PatchPlugins::from_catalog(plugins);
    for patch in &mut cache.patches {
        if patch.audio.selector_category.is_some() {
            continue;
        }
        let info = patch_plugins
            .audio_info_for_ref(&patch.audio.reference)
            .map_err(anyhow::Error::new)?;
        patch.audio.selector_category = info.selector_category(&patch.audio.reference.display);
    }
    Ok(())
}

fn describe_patches(
    plugins: &[CatalogPlugin],
    pairs: &[(String, String)],
) -> Result<Vec<cmrt_core::AudioPatch>> {
    let patch_plugins = cmrt_tui_core::patch_plugins::PatchPlugins::from_catalog(plugins.to_vec());
    pairs
        .iter()
        .map(|(display, _)| {
            let index = patch_plugins
                .index_for_patch(display)
                .map_err(anyhow::Error::new)?;
            let info = patch_plugins
                .audio_info(index)
                .with_context(|| format!("plugin情報がありません: {display}"))?;
            Ok(info.describe_patch(display, None))
        })
        .collect()
}

fn collect_patch_voicings(
    plugins: &[CatalogPlugin],
    patches: &[cmrt_core::AudioPatch],
) -> BTreeMap<String, cmrt_realtime_play::PatchVoicing> {
    let patch_plugins = cmrt_tui_core::patch_plugins::PatchPlugins::from_catalog(plugins.to_vec());
    patches
        .iter()
        .filter_map(|patch| {
            let info = patch_plugins.audio_info_for_ref(&patch.reference).ok()?;
            if info.voicing_source() != cmrt_core::PluginVoicingSource::CatalogMetadata {
                return None;
            }
            let voicing = match patch.voicing {
                cmrt_core::PatchVoicingHint::Known { voicing } => local_voicing(voicing),
                cmrt_core::PatchVoicingHint::ExternalLookup { .. } => return None,
            };
            Some((patch.reference.display.clone(), voicing))
        })
        .collect()
}

fn local_voicing(voicing: cmrt_core::AdapterPatchVoicing) -> cmrt_realtime_play::PatchVoicing {
    match voicing {
        cmrt_core::AdapterPatchVoicing::Mono => cmrt_realtime_play::PatchVoicing::Mono,
        cmrt_core::AdapterPatchVoicing::Poly => cmrt_realtime_play::PatchVoicing::Poly,
        cmrt_core::AdapterPatchVoicing::Unknown => cmrt_realtime_play::PatchVoicing::Unknown,
    }
}

fn validate_catalog_voicings(cache: &CacheFile) -> Result<()> {
    let plugins = cache
        .plugins
        .iter()
        .cloned()
        .map(CatalogPlugin::from)
        .collect::<Vec<_>>();
    let patch_plugins = cmrt_tui_core::patch_plugins::PatchPlugins::from_catalog(plugins);
    for patch in &cache.patches {
        let routed_ref = patch_plugins
            .patch_ref(&patch.audio.reference.display)
            .map_err(anyhow::Error::new)?;
        if routed_ref.plugin != patch.audio.reference.plugin {
            anyhow::bail!(
                "patch catalog cacheのplugin keyが現在のcatalogと一致しません: {}",
                patch.audio.reference.display
            );
        }
        let info = patch_plugins
            .audio_info_for_ref(&patch.audio.reference)
            .map_err(anyhow::Error::new)?;
        let needs_catalog_value =
            info.voicing_source() == cmrt_core::PluginVoicingSource::CatalogMetadata;
        if needs_catalog_value
            && !cache
                .patch_voicings
                .contains_key(&patch.audio.reference.display)
        {
            anyhow::bail!(
                "patch catalog cacheにadapter voicingがありません: {}",
                patch.audio.reference.display
            );
        }
    }
    Ok(())
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
