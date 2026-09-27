//! 前回までのpatch load計測結果を拾い、未計測のpatchだけを選ぶ。

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

use cmrt_tui_core::patch_load::PatchLoadMeasurement;

/// format versionを問わず`patches[].audio.reference.display`と計測fieldだけを読む。
/// fileが無い・壊れている場合は空を返し、全件を計測させる。
pub(super) fn read(path: &Path) -> BTreeMap<String, PatchLoadMeasurement> {
    let Ok(bytes) = fs::read(path) else {
        return BTreeMap::new();
    };
    let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
        return BTreeMap::new();
    };
    let Some(patches) = value.get("patches").and_then(serde_json::Value::as_array) else {
        return BTreeMap::new();
    };
    patches
        .iter()
        .filter_map(|patch| {
            let display = patch
                .pointer("/audio/reference/display")
                .and_then(serde_json::Value::as_str)?;
            let measurement = serde_json::from_value::<PatchLoadMeasurement>(patch.clone()).ok()?;
            Some((display.to_string(), measurement))
        })
        .collect()
}

/// 2回目のload時間が取れているpatchだけを計測済みとする。errorで終わったpatchは計測し直す。
pub(super) fn partition(
    pairs: &[(String, String)],
    mut previous: BTreeMap<String, PatchLoadMeasurement>,
) -> (
    BTreeMap<String, PatchLoadMeasurement>,
    Vec<(String, String)>,
) {
    let mut measured = BTreeMap::new();
    let mut unmeasured = Vec::new();
    for pair in pairs {
        match previous
            .remove(&pair.0)
            .filter(|measurement| measurement.second_load_ms.is_some())
        {
            Some(measurement) => {
                measured.insert(pair.0.clone(), measurement);
            }
            None => unmeasured.push(pair.clone()),
        }
    }
    (measured, unmeasured)
}

#[cfg(test)]
mod tests;
