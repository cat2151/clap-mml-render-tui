//! 弾いた note に音高がついてくる（実測）drum percussion 行の音色を、lead へ移す。
//! 照合は `cmrt_patches::pitch_tracking`（cat-plugin-player・cat-smf-editor と共有）。
//! 表の取得（remote と cache）は各アプリが持つ。

use cmrt_patches::pitch_tracking::{
    pitched_percussion_indices, PitchTrackingTable, TrackingCandidate, PITCHED_PERCUSSION,
};
use cmrt_patches::PatchRoleInput;
use cmrt_tui_core::patch_load::PatchCatalogSnapshot;

/// snapshot の該当音色の category を [`PITCHED_PERCUSSION`] にし、移した件数を返す。
/// 分類索引は作り直さないので、呼んだ後に `rebuild_patch_roles` を呼ぶこと。
pub fn apply_to_snapshot(snapshot: &mut PatchCatalogSnapshot, table: &PitchTrackingTable) -> usize {
    let moved = {
        let plugins = snapshot.patch_plugins();
        let infos: Vec<_> = snapshot
            .audio_patches()
            .iter()
            .map(|patch| plugins.audio_info_for_ref(&patch.reference).ok())
            .collect();
        let candidates: Vec<_> = snapshot
            .audio_patches()
            .iter()
            .zip(&infos)
            .map(|(patch, info)| TrackingCandidate {
                group: info.and_then(|info| info.plugin_id.as_deref()),
                plugin_id: info.and_then(|info| info.plugin_id.as_deref()),
                role: PatchRoleInput {
                    display: &patch.reference.display,
                    normalized_display: &patch.normalized_display,
                    selector_category: patch.selector_category.as_deref(),
                    plugin: info.map(|info| info.name.as_str()),
                },
            })
            .collect();
        pitched_percussion_indices(&candidates, table)
    };
    for &index in &moved {
        snapshot.set_selector_category(index, PITCHED_PERCUSSION);
    }
    moved.len()
}

#[cfg(test)]
mod tests;
