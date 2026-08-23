//! 明示的CLIで構築し、TUIからは読み取り専用で使うpatch catalog cache。

use std::collections::BTreeMap;
use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use cmrt_runtime::{CatalogPlugin, Config, PatchRoles, SkippedCatalogPlugin};
use cmrt_tui_core::patch_load::{PatchCatalogSnapshot, PatchLoadMeasurement};
use serde::{Deserialize, Serialize};

const CACHE_FORMAT_VERSION: u32 = 3;
const CACHE_RELATIVE_PATH: &str = "patch-catalog/catalog.json";
pub const BUILD_COMMAND: &str = "cmrt build-patch-catalog-cache";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildSummary {
    pub path: PathBuf,
    pub patch_count: usize,
    pub plugin_names: Vec<String>,
    pub vvp_voicing_count: usize,
    pub vvp_unknown_count: usize,
    pub measured_load_count: usize,
    pub first_load_failure_count: usize,
    pub second_load_failure_count: usize,
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
    patches: Vec<CachedPatch>,
    plugins: Vec<CachedPlugin>,
    #[serde(default)]
    vvp_voicings: BTreeMap<String, cmrt_realtime_play::PatchVoicing>,
    #[serde(default)]
    catalog_notes: Vec<String>,
}

#[derive(Serialize, Deserialize)]
struct CachedPatch {
    display: String,
    #[serde(flatten)]
    measurement: PatchLoadMeasurement,
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
        patches: pairs
            .into_iter()
            .map(|(display, _)| {
                let measurement = load_measurements
                    .get(&display)
                    .cloned()
                    .with_context(|| format!("patch load計測結果がありません: {display}"))?;
                Ok(CachedPatch {
                    display,
                    measurement,
                })
            })
            .collect::<Result<Vec<_>>>()?,
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
        measured_load_count,
        first_load_failure_count,
        second_load_failure_count,
    })
}

fn collect_patch_load_measurements(
    cfg: &Config,
    pairs: &[(String, String)],
) -> Result<BTreeMap<String, PatchLoadMeasurement>> {
    if pairs.is_empty() {
        return Ok(BTreeMap::new());
    }
    let supervisor =
        cmrt_realtime_play::RealtimePlayServerSupervisor::with_live_instance_count(cfg, 2);
    supervisor
        .start_owned_for_fast_midi()
        .context("patch load計測用realtime play serverを起動できません")?;
    let total = pairs.len();
    let progress_started = Instant::now();
    Ok(measure_patch_loads(
        pairs,
        |instance_id, patch| supervisor.prepare_live_patch(instance_id, Some(patch)),
        Instant::now,
        |index, patch| {
            print!("[{index}/{total}] {patch} ... ");
            let _ = std::io::stdout().flush();
        },
        |index, _patch, measurement| {
            let eta = estimate_eta(progress_started.elapsed(), index, total);
            println!(
                "first={} second={} ETA={}",
                measurement
                    .first_load_error
                    .as_deref()
                    .map_or("ok".to_string(), |error| format!("error: {error}")),
                match (
                    measurement.second_load_ms,
                    measurement.second_load_error.as_deref(),
                ) {
                    (Some(ms), _) => format!("{ms}ms"),
                    (None, Some(error)) => format!("error: {error}"),
                    (None, None) => "error: no measurement".to_string(),
                },
                format_eta(eta),
            );
        },
    ))
}

fn measure_patch_loads<L, N, S, P>(
    pairs: &[(String, String)],
    mut load: L,
    mut now: N,
    mut report_started: S,
    mut report: P,
) -> BTreeMap<String, PatchLoadMeasurement>
where
    L: FnMut(u8, &str) -> Result<()>,
    N: FnMut() -> Instant,
    S: FnMut(usize, &str),
    P: FnMut(usize, &str, &PatchLoadMeasurement),
{
    let mut measurements = BTreeMap::new();
    for (offset, (display, _)) in pairs.iter().enumerate() {
        report_started(offset + 1, display);
        let first_load_error = load(0, display).err().map(|error| format!("{error:#}"));
        let second_started = now();
        let second_result = load(1, display);
        let second_elapsed = now().duration_since(second_started);
        let (second_load_ms, second_load_error) = match second_result {
            Ok(()) => (
                Some(u64::try_from(second_elapsed.as_millis()).unwrap_or(u64::MAX)),
                None,
            ),
            Err(error) => (None, Some(format!("{error:#}"))),
        };
        let measurement = PatchLoadMeasurement {
            second_load_ms,
            first_load_error,
            second_load_error,
        };
        report(offset + 1, display, &measurement);
        measurements.insert(display.clone(), measurement);
    }
    measurements
}

fn estimate_eta(elapsed: Duration, completed: usize, total: usize) -> Duration {
    let remaining = total.saturating_sub(completed);
    if completed == 0 || remaining == 0 {
        return Duration::ZERO;
    }
    elapsed.mul_f64(remaining as f64 / completed as f64)
}

fn format_eta(eta: Duration) -> String {
    let seconds = eta.as_secs();
    format!("{}分{:02}秒", seconds / 60, seconds % 60)
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
    let cache: CacheFile = serde_json::from_value(value)
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
    if let Some(display) = cache
        .patches
        .iter()
        .map(|patch| &patch.display)
        .filter(|display| cmrt_core::is_vvp_patch_path(display))
        .find(|display| !cache.vvp_voicings.contains_key(*display))
    {
        anyhow::bail!("patch catalog cacheにVVP voicingがありません: {display}");
    }
    if let Some(display) = cache
        .patches
        .iter()
        .find(|patch| {
            patch.measurement.second_load_ms.is_none()
                && patch.measurement.second_load_error.is_none()
        })
        .map(|patch| &patch.display)
    {
        anyhow::bail!("patch catalog cacheに2回目のload計測結果がありません: {display}");
    }
    let vvp_voicings = cache.vvp_voicings;
    let mut load_measurements = BTreeMap::new();
    let pairs = cache
        .patches
        .into_iter()
        .map(|patch| {
            let display = patch.display;
            let lower = display.to_lowercase();
            load_measurements.insert(display.clone(), patch.measurement);
            (display, lower)
        })
        .collect();
    Ok(LoadedPatchCatalogCache {
        snapshot: PatchCatalogSnapshot::new(
            pairs,
            cache.plugins.into_iter().map(CatalogPlugin::from).collect(),
            cache.catalog_notes,
            load_measurements,
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
