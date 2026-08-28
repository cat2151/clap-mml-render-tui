//! ホストが持つ file cache 由来の patch catalog から、overlay へ渡す一式を作る。
//!
//! overlay を開ける画面は app（notepad / keyboard / grid sequencer）と DAW の 2 系統あり、
//! どちらも `cmrt_tui_core::patch_load::PatchLoadState` を共有している。**変換をここ 1 か所に
//! 置く**のは、`Loading` / `Err` のときに何を渡すか（空の一覧か、既定の Role 索引か）が
//! 画面ごとに食い違うと、同じ overlay が画面によって違う一覧を出すため。

use std::collections::BTreeMap;

use cmrt_patches::PatchRoleIndex;
use cmrt_tui_core::patch_load::{PatchLoadMeasurement, PatchLoadState};

use super::PatchCatalogEntry;
use crate::PatchCatalogSnapshot;

/// overlay を開くときに渡す、音色一覧まわりの 3 点セット。
pub struct HostPatchCatalog {
    pub catalog: PatchCatalogSnapshot,
    pub patch_role_index: PatchRoleIndex,
    pub load_measurements: BTreeMap<String, PatchLoadMeasurement>,
}

/// ホストの `PatchLoadState` を overlay 向けへ変換する。
///
/// `Loading` / `Err` はそのまま overlay 側の同名の状態になり、overlay は
/// 「一覧が来たら開き直す」予約として扱う（`Ctrl+T` の Loading 予約）。
pub fn host_patch_catalog(state: &PatchLoadState) -> HostPatchCatalog {
    match state {
        PatchLoadState::Loading => HostPatchCatalog {
            catalog: PatchCatalogSnapshot::Loading,
            patch_role_index: PatchRoleIndex::default(),
            load_measurements: BTreeMap::new(),
        },
        PatchLoadState::Ready(snapshot) => HostPatchCatalog {
            catalog: PatchCatalogSnapshot::Ready(catalog_entries(snapshot)),
            patch_role_index: snapshot.patch_roles().clone(),
            load_measurements: snapshot.load_measurements().clone(),
        },
        PatchLoadState::Err(error) => HostPatchCatalog {
            catalog: PatchCatalogSnapshot::Error(error.clone()),
            patch_role_index: PatchRoleIndex::default(),
            load_measurements: BTreeMap::new(),
        },
    }
}

/// selector 行を作る。plugin 情報が欠けている（`audio_patches` が pairs と揃わない）
/// snapshot では表示名だけの行へ落とす。
fn catalog_entries(
    snapshot: &cmrt_tui_core::patch_load::PatchCatalogSnapshot,
) -> Vec<PatchCatalogEntry> {
    if snapshot.audio_patches().len() != snapshot.pairs().len() {
        return snapshot
            .pairs()
            .iter()
            .map(|(display, normalized)| {
                PatchCatalogEntry::new(display.clone(), normalized.clone(), String::new(), None)
            })
            .collect();
    }
    snapshot
        .audio_patches()
        .iter()
        .map(|patch| {
            let plugin_sort_key = snapshot
                .patch_plugins()
                .audio_info_for_ref(&patch.reference)
                .map(|plugin| plugin.name.clone())
                .unwrap_or_else(|_| patch.reference.plugin.to_string());
            PatchCatalogEntry::new(
                patch.reference.display.clone(),
                patch.normalized_display.clone(),
                plugin_sort_key,
                patch.selector_category.clone(),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests;
