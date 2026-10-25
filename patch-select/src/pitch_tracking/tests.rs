use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::Arc,
};

use cmrt_patches::pitch_tracking::{PitchTrackingTable, PITCHED_PERCUSSION, PITCH_TRACKING_FILE};
use cmrt_patches::PatchRole;
use cmrt_runtime::{DEXED_PLUGIN_ID, SURGE_XT_PLUGIN_ID};
use cmrt_tui_core::patch_load::PatchLoadState;
use cmrt_tui_core::patch_plugins::CatalogPlugin;

use super::*;
use crate::{host_patch_catalog, FilterGroup, PatchCatalogEntry, PreparedPresets};

const VAST_PLUGIN_ID: &str = "com.vastdynamics.VAST2";

fn table(entries: &[(&str, &str, bool)]) -> PitchTrackingTable {
    let entries: Vec<_> = entries
        .iter()
        .map(|(plugin_id, display, tracks_keyboard)| {
            serde_json::json!({
                "plugin_id": plugin_id,
                "display": display,
                "tracks_keyboard": tracks_keyboard,
                "observations": [{ "note": 48, "frequency_hz": null }],
            })
        })
        .collect();
    let json = serde_json::json!({ "format_version": 1, "entries": entries });
    PitchTrackingTable::parse(json.to_string().as_bytes()).unwrap()
}

fn catalog_plugin(name: &str, plugin_id: &str) -> CatalogPlugin {
    CatalogPlugin {
        name: name.to_string(),
        plugin_path: format!("/clap/{name}.clap"),
        plugin_id: Some(plugin_id.to_string()),
        base: cmrt_runtime::PatchBase::None,
        dirs: Vec::new(),
        resolved_patches: None,
        source_notices: Vec::new(),
    }
}

/// (plugin 名, plugin_id, display, selector_category) から snapshot を作る。
fn snapshot(patches: &[(&str, &str, &str, Option<&str>)]) -> PatchCatalogSnapshot {
    let audio: Vec<_> = patches
        .iter()
        .map(|(name, plugin_id, display, category)| {
            let mut patch = cmrt_core::AudioPluginInfo::new(
                *name,
                format!("/clap/{name}.clap"),
                Some(plugin_id.to_string()),
                None,
            )
            .describe_patch(display, None);
            patch.selector_category = category.map(str::to_string);
            patch
        })
        .collect();
    let pairs = audio
        .iter()
        .map(|patch| {
            (
                patch.reference.display.clone(),
                patch.normalized_display.clone(),
            )
        })
        .collect();
    let mut plugins: Vec<_> = patches
        .iter()
        .map(|(name, plugin_id, _, _)| catalog_plugin(name, plugin_id))
        .collect();
    plugins.dedup_by(|a, b| a.name == b.name);
    PatchCatalogSnapshot::new(pairs, audio, plugins, Vec::new(), Default::default())
}

fn group_index(role: PatchRole) -> usize {
    FilterGroup::ALL
        .iter()
        .position(|g| g.role() == Some(role))
        .unwrap()
}

/// selector の欄 `label`（Role `role`）に並ぶ音色の display。
fn row(
    prepared: &PreparedPresets,
    all: &[PatchCatalogEntry],
    role: PatchRole,
    label: &str,
) -> BTreeSet<String> {
    let preset = prepared
        .for_role(group_index(role))
        .iter()
        .find(|p| p.label == label)
        .unwrap_or_else(|| panic!("no row {label}"));
    preset
        .matches
        .iter()
        .map(|&index| all[index].display().to_string())
        .collect()
}

/// アプリと同じ経路（snapshot → selector 行）で欄を組む。
fn selector_rows(snapshot: PatchCatalogSnapshot) -> (PreparedPresets, Vec<PatchCatalogEntry>) {
    let host = host_patch_catalog(&PatchLoadState::Ready(Arc::new(snapshot)));
    let crate::PatchCatalogSnapshot::Ready(all) = host.catalog else {
        panic!("catalog is not ready");
    };
    let prepared = PreparedPresets::build(&all, &[], &host.patch_role_index).unwrap();
    (prepared, all)
}

fn set(displays: &[&str]) -> BTreeSet<String> {
    displays.iter().map(|d| d.to_string()).collect()
}

#[test]
fn tracking_percussion_moves_for_every_plugin_and_other_rows_stay() {
    let mut snapshot = snapshot(&[
        ("Dexed", DEXED_PLUGIN_ID, "bank.syx/01 PERCSYN", None),
        (
            "Surge XT",
            SURGE_XT_PLUGIN_ID,
            "Percussion/Drum One.fxp",
            Some("Percussion"),
        ),
        ("Vaporizer2", VAST_PLUGIN_ID, "DR Mystery.vvp", Some("Drum")),
        ("Dexed", DEXED_PLUGIN_ID, "bank.syx/02 CONGA perc", None),
        ("Dexed", DEXED_PLUGIN_ID, "bank.syx/03 kick", None),
        ("Dexed", DEXED_PLUGIN_ID, "bank.syx/04 snare", None),
        ("Dexed", DEXED_PLUGIN_ID, "bank.syx/05 hat", None),
        ("Dexed", DEXED_PLUGIN_ID, "bank.syx/06 plain lead", None),
    ]);
    let table = table(&[
        (DEXED_PLUGIN_ID, "bank.syx/01 PERCSYN", true),
        (SURGE_XT_PLUGIN_ID, "Percussion/Drum One.fxp", true),
        (VAST_PLUGIN_ID, "DR Mystery.vvp", true),
        (DEXED_PLUGIN_ID, "bank.syx/02 CONGA perc", false),
        (DEXED_PLUGIN_ID, "bank.syx/03 kick", true),
        (DEXED_PLUGIN_ID, "bank.syx/04 snare", true),
        (DEXED_PLUGIN_ID, "bank.syx/05 hat", true),
        (DEXED_PLUGIN_ID, "bank.syx/06 plain lead", true),
    ]);
    assert_eq!(apply_to_snapshot(&mut snapshot, &table), 3);
    snapshot.rebuild_patch_roles(&[]);
    let (prepared, all) = selector_rows(snapshot);

    assert_eq!(
        row(&prepared, &all, PatchRole::Lead, PITCHED_PERCUSSION),
        set(&[
            "bank.syx/01 PERCSYN",
            "Percussion/Drum One.fxp",
            "DR Mystery.vvp",
        ])
    );
    assert_eq!(
        row(&prepared, &all, PatchRole::Drum, "percussion"),
        set(&["bank.syx/02 CONGA perc"])
    );
    assert!(row(&prepared, &all, PatchRole::Drum, "kick").contains("bank.syx/03 kick"));
    assert!(row(&prepared, &all, PatchRole::Drum, "snare").contains("bank.syx/04 snare"));
    assert!(row(&prepared, &all, PatchRole::Drum, "hi-hat").contains("bank.syx/05 hat"));
    assert!(!row(&prepared, &all, PatchRole::Lead, PITCHED_PERCUSSION)
        .contains("bank.syx/06 plain lead"));
}

/// 実 catalog（cmrt の patch catalog cache）と兄弟 repo の表で、表で true の drum percussion
/// 音色がすべて lead へ移り（名前が mallet・bell のものはその欄、残りは pitched percussion 欄）、
/// percussion 欄から消えることを確かめる。
#[test]
#[ignore = "読むだけ。cmrt の patch catalog cache と ../cat-music-patterns の表が要る（音・画面は使わない）"]
fn installed_tracking_percussion_all_moves_to_lead() {
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../cat-music-patterns")
        .join(PITCH_TRACKING_FILE);
    let table = PitchTrackingTable::parse(&std::fs::read(file).unwrap()).unwrap();
    let (snapshot, _) = cmrt_tui_core::patch_catalog_cache::load()
        .unwrap()
        .into_parts();
    let plugin_of: BTreeMap<String, String> = snapshot
        .audio_patches()
        .iter()
        .filter_map(|patch| {
            let info = snapshot
                .patch_plugins()
                .audio_info_for_ref(&patch.reference)
                .ok()?;
            Some((patch.reference.display.clone(), info.plugin_id.clone()?))
        })
        .collect();
    let key = |display: &String| (plugin_of[display].clone(), display.clone());

    let (before, before_all) = selector_rows(snapshot.clone());
    let percussion: BTreeSet<_> = row(&before, &before_all, PatchRole::Drum, "percussion")
        .iter()
        .map(key)
        .collect();
    let tracking: BTreeSet<_> = table
        .tracking()
        .map(|(plugin_id, display)| (plugin_id.to_string(), display.to_string()))
        .collect();

    let mut applied = snapshot;
    let moved = apply_to_snapshot(&mut applied, &table);
    applied.rebuild_patch_roles(&[]);
    let (after, after_all) = selector_rows(applied);
    let pitched: BTreeSet<_> = row(&after, &after_all, PatchRole::Lead, PITCHED_PERCUSSION)
        .iter()
        .map(key)
        .collect();
    let lead_row = |label| -> BTreeSet<_> {
        row(&after, &after_all, PatchRole::Lead, label)
            .iter()
            .map(key)
            .collect()
    };
    let expected: BTreeSet<_> = percussion.intersection(&tracking).cloned().collect();
    let named: BTreeSet<_> = lead_row("mallet")
        .union(&lead_row("bell"))
        .filter(|patch| expected.contains(*patch))
        .cloned()
        .collect();
    let left: BTreeSet<_> = row(&after, &after_all, PatchRole::Drum, "percussion")
        .iter()
        .map(key)
        .collect();

    let mut counts: BTreeMap<&str, (usize, usize)> = BTreeMap::new();
    for (plugin_id, _) in &tracking {
        counts.entry(plugin_id).or_default().0 += 1;
    }
    for (plugin_id, _) in &pitched {
        counts.entry(plugin_id).or_default().1 += 1;
    }
    println!(
        "patches {}, moved {moved}, drum percussion {} -> {}",
        after_all.len(),
        percussion.len(),
        left.len()
    );
    for (plugin_id, (in_table, in_row)) in &counts {
        println!("{plugin_id}: table true {in_table}, pitched percussion {in_row}");
    }
    for (plugin_id, display) in &named {
        println!("mallet or bell by name: {plugin_id} {display}");
    }
    let in_lead: BTreeSet<_> = pitched.union(&named).cloned().collect();
    for (plugin_id, display) in tracking.difference(&in_lead) {
        let reason = if plugin_of.get(display) == Some(plugin_id) {
            "not on the drum percussion row in cmrt"
        } else {
            "not in cmrt's catalog"
        };
        println!("not moved: {plugin_id} {display}: {reason}");
    }
    assert_eq!(in_lead, expected);
    assert_eq!(moved, expected.len());
    assert!(left.is_disjoint(&tracking));
}
