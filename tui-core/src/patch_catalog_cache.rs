//! 明示的CLIで構築し、TUIからは読み取り専用で使うpatch catalog cacheの形式とreader。
//!
//! 構築（全patch load計測）は app に残し、ここは読むだけ。cat-plugin-player が
//! TUI アプリ本体（app）に依存せずに同じ catalog を読めるよう、app から分けてある。

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cmrt_runtime::CatalogPlugin;
use serde::{Deserialize, Serialize};

use crate::patch_load::{PatchCatalogSnapshot, PatchLoadMeasurement};
use crate::patch_plugins::PatchPlugins;

pub const CACHE_FORMAT_VERSION: u32 = 5;
pub const CACHE_RELATIVE_PATH: &str = "patch-catalog/catalog.json";

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
pub struct CacheFile {
    pub format_version: u32,
    pub patches: Vec<CachedPatch>,
    pub plugins: Vec<CachedPlugin>,
    #[serde(default)]
    pub patch_voicings: BTreeMap<String, cmrt_realtime_play::PatchVoicing>,
    #[serde(default)]
    pub catalog_notes: Vec<String>,
}

#[derive(Serialize, Deserialize)]
pub struct CachedPatch {
    /// server shared coreが解釈したplugin key・表示名・分類・voicing。
    pub audio: cmrt_core::AudioPatch,
    #[serde(flatten)]
    pub measurement: PatchLoadMeasurement,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct CachedPlugin {
    pub name: String,
    pub plugin_path: String,
    pub plugin_id: Option<String>,
    pub base: cmrt_runtime::PatchBase,
    pub dirs: Vec<String>,
    #[serde(default)]
    pub source_notices: Vec<String>,
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
    cmrt_runtime::config_app_dir().map(|dir| dir.join(CACHE_RELATIVE_PATH))
}

pub fn load() -> Result<LoadedPatchCatalogCache> {
    let path = cache_file_path().context("patch catalog cacheの保存先を取得できません")?;
    load_from(&path)
}

/// `load()` と同じ検査で、指定したパスの catalog を読む。
pub fn load_from(path: &Path) -> Result<LoadedPatchCatalogCache> {
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
    let patch_plugins = PatchPlugins::from_catalog(plugins);
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

fn validate_catalog_voicings(cache: &CacheFile) -> Result<()> {
    let plugins = cache
        .plugins
        .iter()
        .cloned()
        .map(CatalogPlugin::from)
        .collect::<Vec<_>>();
    let patch_plugins = PatchPlugins::from_catalog(plugins);
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
