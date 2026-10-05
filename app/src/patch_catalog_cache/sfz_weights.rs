//! `.sfz` の patch が参照する sample の数と総容量を catalog へ記録し、分布を表示する。

use std::collections::BTreeMap;
use std::path::Path;

use cmrt_runtime::CatalogPlugin;
use cmrt_tui_core::patch_load::{PatchLoadMeasurement, HEAVY_OFFLINE_LOAD_BYTES};
use cmrt_tui_core::patch_plugins::PatchPlugins;

/// 重い順に表示する件数。
const TOP_COUNT: usize = 20;

/// `.sfz` の patch を毎回数え直して上書きする。集計できなかった `.sfz` の display を返す。
pub(super) fn record(
    plugins: &[CatalogPlugin],
    measurements: &mut BTreeMap<String, PatchLoadMeasurement>,
) -> Vec<String> {
    let patch_plugins = PatchPlugins::from_catalog(plugins.to_vec());
    let mut failed = Vec::new();
    let total = measurements
        .keys()
        .filter(|display| cmrt_core::is_sfz_patch_path(display))
        .count();
    let mut current = 0;
    for (display, measurement) in measurements.iter_mut() {
        let weight = if cmrt_core::is_sfz_patch_path(display) {
            current += 1;
            super::report_progress(format!(
                "  [{current}/{total}] SFZ sample の数と容量を集計します: {display}"
            ));
            let weight = weigh(&patch_plugins, display);
            if weight.is_none() {
                failed.push(display.clone());
            }
            weight
        } else {
            None
        };
        measurement.sfz_sample_files = weight.map(|weight| weight.files);
        measurement.sfz_sample_bytes = weight.map(|weight| weight.bytes);
    }
    failed
}

fn weigh(patch_plugins: &PatchPlugins, display: &str) -> Option<cmrt_core::SfzSampleWeight> {
    let plugin = patch_plugins.for_patch(display).ok()?;
    let path = plugin.base.resolve(display);
    cmrt_core::sfz_sample_weight(Path::new(&path)).ok()
}

/// 集計できた `.sfz` の容量分布と、重い順の上位。MB は 10^6 byte。
pub(super) fn distribution_lines(
    measurements: &BTreeMap<String, PatchLoadMeasurement>,
    failed: &[String],
) -> Vec<String> {
    let mut weighed = measurements
        .iter()
        .filter_map(|(display, measurement)| {
            Some((
                measurement.sfz_sample_bytes?,
                measurement.sfz_sample_files?,
                display.as_str(),
                measurement.is_heavy_offline_load(),
            ))
        })
        .collect::<Vec<_>>();
    weighed.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.2.cmp(b.2)));
    let mut ascending = weighed.iter().map(|entry| entry.0).collect::<Vec<_>>();
    ascending.reverse();
    let heavy = weighed.iter().filter(|entry| entry.3).count();
    let mut lines = vec![format!(
        "sfz sample集計: sfz={} 集計失敗={} p50={} p90={} p99={} 重い(>={})={}",
        weighed.len(),
        failed.len(),
        percentile_mb(&ascending, 50),
        percentile_mb(&ascending, 90),
        percentile_mb(&ascending, 99),
        mb(HEAVY_OFFLINE_LOAD_BYTES),
        heavy,
    )];
    lines.extend(
        failed
            .iter()
            .map(|display| format!("  集計失敗: {display}")),
    );
    if !weighed.is_empty() {
        lines.push(format!("sfz sample総容量の上位{TOP_COUNT}件:"));
        lines.extend(
            weighed
                .iter()
                .take(TOP_COUNT)
                .map(|(bytes, files, display, _)| {
                    format!("  {files:>5} files {:>10}  {display}", mb(*bytes))
                }),
        );
    }
    lines
}

/// 昇順の値から nearest-rank で百分位を引く。空なら `-`。
fn percentile_mb(ascending: &[u64], percent: usize) -> String {
    if ascending.is_empty() {
        return "-".to_string();
    }
    let rank = (ascending.len() * percent).div_ceil(100).max(1);
    mb(ascending[rank - 1])
}

fn mb(bytes: u64) -> String {
    format!("{:.1}MB", bytes as f64 / 1_000_000.0)
}

#[cfg(test)]
mod tests;
