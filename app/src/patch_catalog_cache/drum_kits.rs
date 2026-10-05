//! 鍵ごとに別の音を割り当てた kit（drum kit・効果音 kit）かを catalog へ記録する。
//!
//! `.sfz` は鍵割り当てから、Floe preset は `percussion` タグから判定する。

use std::collections::BTreeMap;
use std::path::Path;

use cmrt_runtime::CatalogPlugin;
use cmrt_tui_core::patch_load::PatchLoadMeasurement;
use cmrt_tui_core::patch_plugins::PatchPlugins;

/// patch ごとに毎回判定し直して上書きする。読めない patch は kit でない扱い。
pub(super) fn record(
    plugins: &[CatalogPlugin],
    measurements: &mut BTreeMap<String, PatchLoadMeasurement>,
) -> Vec<String> {
    let patch_plugins = PatchPlugins::from_catalog(plugins.to_vec());
    let mut diagnostics = Vec::new();
    let total = measurements
        .keys()
        .filter(|display| {
            cmrt_core::is_sfz_patch_path(display) || cmrt_core::is_floe_preset_path(display)
        })
        .count();
    let mut current = 0;
    for (display, measurement) in measurements.iter_mut() {
        if cmrt_core::is_sfz_patch_path(display) || cmrt_core::is_floe_preset_path(display) {
            current += 1;
            super::report_progress(format!(
                "  [{current}/{total}] Drum kit か判定します: {display}"
            ));
        }
        measurement.drum_kit = is_drum_kit(&patch_plugins, display);
        // load 計測を再利用しても、割当は必ず今回の file から取得する。
        measurement.drum_kit_notes = None;
        if measurement.drum_kit {
            super::report_progress(format!(
                "  Drum kit の割当 note 一覧を抽出します: {display}"
            ));
            match note_assignments(&patch_plugins, display) {
                Ok(notes) => measurement.drum_kit_notes = Some(notes),
                Err(error) => diagnostics.push(format!(
                    "drum kit note 一覧の抽出失敗: {display}: {error:#}"
                )),
            }
        }
    }
    diagnostics
}

fn note_assignments(patch_plugins: &PatchPlugins, display: &str) -> anyhow::Result<Vec<u8>> {
    let plugin = patch_plugins.for_patch(display)?;
    let path = plugin.base.resolve(display);
    let path = Path::new(&path);
    if cmrt_core::is_sfz_patch_path(display) {
        cmrt_core::sfz_note_assignments(path)
    } else {
        cmrt_core::floe_note_assignments(path)
    }
}

fn is_drum_kit(patch_plugins: &PatchPlugins, display: &str) -> bool {
    let sfz = cmrt_core::is_sfz_patch_path(display);
    if !sfz && !cmrt_core::is_floe_preset_path(display) {
        return false;
    }
    let Ok(plugin) = patch_plugins.for_patch(display) else {
        return false;
    };
    let path = plugin.base.resolve(display);
    let path = Path::new(&path);
    if sfz {
        cmrt_core::sfz_is_drum_kit(path).unwrap_or(false)
    } else {
        cmrt_core::floe_preset_is_percussion(path).unwrap_or(false)
    }
}
