//! 明示的CLIで構築し、TUIからは読み取り専用で使うpatch catalog cache。

use std::fs;
use std::path::PathBuf;

use anyhow::{Context, Result};
use cmrt_runtime::{CatalogPlugin, Config, SkippedCatalogPlugin};
#[cfg(test)]
use cmrt_tui_core::patch_load::PatchLoadMeasurement;

use measurements::collect_patch_load_measurements;
#[cfg(test)]
use measurements::{estimate_eta, format_eta, measure_patch_loads};

mod cache_writer;
mod drum_kits;
mod measurement_log;
mod measurements;
mod patch_metadata;
mod previous_measurements;
mod progress;
mod sfz_weights;
mod source_cache;

use cache_writer::{replace_file, write_cache};
use patch_metadata::{collect_patch_voicings, describe_patches};
pub use progress::report as report_progress;

pub use cmrt_tui_core::patch_catalog_cache::{load_from, LoadedPatchCatalogCache};
use cmrt_tui_core::patch_catalog_cache::{
    CacheFile, CachedPatch, CachedPlugin, CACHE_FORMAT_VERSION, CACHE_RELATIVE_PATH,
};

pub const BUILD_COMMAND: &str = "cmrt build-patch-catalog-cache";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BuildSummary {
    pub path: PathBuf,
    pub source_path: PathBuf,
    pub patch_count: usize,
    pub plugin_names: Vec<String>,
    pub catalog_voicing_count: usize,
    pub catalog_unknown_count: usize,
    pub measured_load_count: usize,
    pub reused_load_count: usize,
    pub first_load_failure_count: usize,
    pub second_load_failure_count: usize,
}

/// tests が thread ごとの app dir を差し込めるよう、tui-core 側でなく app の config dir を使う。
pub fn cache_file_path() -> Option<PathBuf> {
    crate::config::config_app_dir().map(|dir| dir.join(CACHE_RELATIVE_PATH))
}

pub fn build_and_save(cfg: &Config) -> Result<BuildSummary> {
    if cfg.plugin_path.trim().is_empty() {
        anyhow::bail!("plugin_pathが空のためpatch catalog cacheを構築できません");
    }
    let path = cache_file_path().context("patch catalog cacheの保存先を取得できません")?;
    let source_path =
        source_cache::cache_file_path().context("catalog source cacheの保存先を取得できません")?;
    let (plugins, skipped) = progress::run("plugin ごとの音色 source を探索", || {
        Ok(cmrt_runtime::catalog_plugins_detailed_with_progress(
            cfg,
            |name| report_progress(format!("  {name}: 音色 source を探索します…")),
        ))
    })?;
    // scanが終わった時点でserver用の小さい結果を確定する。後続の全patch load計測が
    // 失敗しても、次のserver起動で同じcatalog scanを繰り返させない。
    progress::run(
        &format!("音色 source cache を保存: {}", source_path.display()),
        || source_cache::write(&source_path, &plugins),
    )?;
    let listing = progress::run(
        "plugin ごとの音色一覧を収集・重複整理",
        || {
            cmrt_tui_core::patches::collect_patch_listing_from_catalog_with_progress(
                &plugins,
                |plugin, dir| {
                    let source = dir.map_or_else(
                        || {
                            format!(
                                "解決済み一覧 {}件",
                                plugin.resolved_patches.as_ref().map_or(0, Vec::len)
                            )
                        },
                        str::to_string,
                    );
                    report_progress(format!("  {}: 音色一覧を収集します: {source}", plugin.name));
                },
            )
        },
    )?;
    let pairs = listing.pairs;
    let mut audio_patches = progress::run(
        &format!("音色 {}件の分類情報を作成", pairs.len()),
        || describe_patches(&plugins, &pairs),
    )?;
    for audio in &mut audio_patches {
        audio.merged = listing.merged.get(&audio.reference.display).cloned();
    }
    let patch_voicings = progress::run("音色の mono/poly 情報を集計", || {
        Ok(collect_patch_voicings(&plugins, &audio_patches))
    })?;
    let catalog_unknown_count = patch_voicings
        .values()
        .filter(|voicing| **voicing == cmrt_realtime_play::PatchVoicing::Unknown)
        .count();
    let log_path = measurement_log::path_next_to(&path);
    let previous = progress::run(
        "既存 catalog と中断時ログから load 計測結果を復元",
        || {
            let mut previous = previous_measurements::read(&path);
            previous.extend(measurement_log::read(&log_path));
            Ok(previous)
        },
    )?;
    let (mut load_measurements, unmeasured) = previous_measurements::partition(&pairs, previous);
    let reused_load_count = load_measurements.len();
    report_progress(format!(
        "patch全件={} 計測済み(再利用)={} 未計測={}",
        pairs.len(),
        reused_load_count,
        unmeasured.len()
    ));
    for (display, _) in &unmeasured {
        println!("  未計測: {display}");
    }
    let mut log = progress::run(
        &format!("load 計測ログを準備: {}", log_path.display()),
        || measurement_log::Writer::open(&log_path),
    )?;
    load_measurements.extend(progress::run(
        "未計測音色の load 時間を計測",
        || collect_patch_load_measurements(cfg, &unmeasured, &mut log),
    )?);
    let sfz_weight_failures = progress::run(
        "SFZ が参照する sample の数と容量を集計",
        || Ok(sfz_weights::record(&plugins, &mut load_measurements)),
    )?;
    let drum_note_failures = progress::run(
        "Drum kit を判定し、割当 note 一覧を抽出",
        || Ok(drum_kits::record(&plugins, &mut load_measurements)),
    )?;
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
    let mut catalog_notes = catalog_notes(&plugins, &skipped);
    catalog_notes.extend(drum_note_failures);
    let cache = progress::run("catalog の保存データを組み立て", || {
        Ok(CacheFile {
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
        })
    })?;
    progress::run(
        &format!("patch catalog cache を保存: {}", path.display()),
        || write_cache(&path, &cache),
    )?;
    // 残っても次回の構築で同じ結果として読まれるだけなので、消せなくても失敗にしない。
    let _ = fs::remove_file(&log_path);
    for line in sfz_weights::distribution_lines(&load_measurements, &sfz_weight_failures) {
        println!("{line}");
    }
    Ok(BuildSummary {
        path,
        source_path,
        patch_count: cache.patches.len(),
        plugin_names: plugins.into_iter().map(|plugin| plugin.name).collect(),
        catalog_voicing_count: cache.patch_voicings.len(),
        catalog_unknown_count,
        measured_load_count,
        reused_load_count,
        first_load_failure_count,
        second_load_failure_count,
    })
}

pub fn load() -> Result<LoadedPatchCatalogCache> {
    let path = cache_file_path().context("patch catalog cacheの保存先を取得できません")?;
    load_from(&path)
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

#[cfg(test)]
mod tests;
