use std::collections::BTreeMap;

use super::*;

/// 組み込み Preset の欄（`ALL`・`★ Favorite`・`Drum kit`・ユーザー追加を除く）の名前ごとに、
/// 2 欄以上へ並んだ音色の数を返す。key は欄名を `+` でつないだもの。
fn row_overlaps(prepared: &PreparedPresets, all: &[PatchCatalogEntry]) -> BTreeMap<String, usize> {
    let mut rows_by_patch = vec![Vec::new(); all.len()];
    for (role, group) in FilterGroup::ALL.iter().enumerate().skip(1) {
        for preset in prepared.for_role(role) {
            if preset.pattern.is_none()
                || preset.is_favorite
                || preset.is_drum_kit
                || preset.is_user
            {
                continue;
            }
            let row = format!("{} › {}", group.label(), preset.label);
            for &index in preset.matches.iter() {
                rows_by_patch[index].push(row.clone());
            }
        }
    }
    let mut overlaps = BTreeMap::new();
    for rows in rows_by_patch.into_iter().filter(|rows| rows.len() > 1) {
        *overlaps.entry(rows.join(" + ")).or_insert(0) += 1;
    }
    overlaps
}

fn prepare(patches: &[PatchCatalogEntry]) -> PreparedPresets {
    let roles = build_role_index(patches, &[]);
    PreparedPresets::build(patches, &[], &roles).unwrap()
}

fn rows_of(prepared: &PreparedPresets, index: usize) -> Vec<String> {
    let mut rows = Vec::new();
    for (role, group) in FilterGroup::ALL.iter().enumerate().skip(1) {
        for preset in prepared.for_role(role).iter().skip(1) {
            if preset.matches.contains(&index) {
                rows.push(format!("{} › {}", group.label(), preset.label));
            }
        }
    }
    rows
}

#[test]
fn tubular_bell_is_mallet_only() {
    let patches = vec![
        entry("Tubular Bell", "Surge XT", None),
        entry("Bell Choir", "Surge XT", None),
        entry("Glock Bell", "Dexed", None),
    ];
    let prepared = prepare(&patches);
    assert_eq!(rows_of(&prepared, 0), ["Lead / melody › mallet"]);
    assert_eq!(rows_of(&prepared, 2), ["Lead / melody › mallet"]);
    assert!(row_overlaps(&prepared, &patches).is_empty());
}

#[test]
fn bell_without_mallet_word_stays_in_bell() {
    let patches = vec![entry("Bell Lead 2", "Surge XT", Some("Bells"))];
    let prepared = prepare(&patches);
    assert_eq!(rows_of(&prepared, 0).len(), 1);
}

#[test]
fn measured_percussion_named_mallet_or_bell_goes_to_that_row_only() {
    let patches = vec![
        entry("Bank/01 TUBULAR BELL", "Dexed", Some("pitched percussion")),
        entry("Bank/02 PERC BELL", "Dexed", Some("pitched percussion")),
        entry("Bank/03 PERCSYN", "Dexed", Some("pitched percussion")),
    ];
    let prepared = prepare(&patches);
    assert_eq!(rows_of(&prepared, 0), ["Lead / melody › mallet"]);
    assert_eq!(rows_of(&prepared, 1), ["Lead / melody › bell"]);
    assert_eq!(
        rows_of(&prepared, 2),
        ["Lead / melody › pitched percussion"]
    );
}

#[test]
fn every_builtin_row_of_a_role_is_disjoint() {
    let patches = vec![
        entry("String Pad", "Surge XT", None),
        entry("Organ Keys", "Dexed", None),
        entry("Brass Strings", "Dexed", None),
        entry("Flute Lead", "Dexed", None),
        entry("Sax Bell", "Dexed", None),
    ];
    let prepared = prepare(&patches);
    assert!(row_overlaps(&prepared, &patches).is_empty());
    for (index, patch) in patches.iter().enumerate() {
        assert_eq!(rows_of(&prepared, index).len(), 1, "{}", patch.display());
    }
}

/// 実 catalog（cmrt の patch catalog cache）に、アプリと同じく兄弟 repo の音高追従表を
/// 当てたうえで、各欄が並べる音色を列挙して重複を数える。
#[test]
#[ignore = "読むだけ。cmrt の patch catalog cache と ../cat-music-patterns の表が要る（音・画面は使わない）"]
fn installed_patches_belong_to_one_preset_row() {
    use crate::pitch_tracking::apply_to_snapshot;
    use cmrt_patches::pitch_tracking::{PitchTrackingTable, PITCH_TRACKING_FILE};
    let (mut snapshot, _) = cmrt_tui_core::patch_catalog_cache::load()
        .unwrap()
        .into_parts();
    let table = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../cat-music-patterns")
        .join(PITCH_TRACKING_FILE);
    let table = PitchTrackingTable::parse(&std::fs::read(table).unwrap()).unwrap();
    println!(
        "moved by pitch tracking: {}",
        apply_to_snapshot(&mut snapshot, &table)
    );
    snapshot.rebuild_patch_roles(&[]);
    let host = crate::host_patch_catalog(&cmrt_tui_core::patch_load::PatchLoadState::Ready(
        Arc::new(snapshot),
    ));
    let crate::PatchCatalogSnapshot::Ready(mut all) = host.catalog else {
        panic!("catalog is not ready");
    };
    crate::sort_for_selector(&mut all);
    let prepared = prepare_presets(
        &all,
        &[],
        &host.patch_role_index,
        &[],
        &host.load_measurements,
    );
    let overlaps = row_overlaps(&prepared, &all);
    let mut per_row: BTreeMap<String, usize> = BTreeMap::new();
    let mut without_row = BTreeMap::<&str, usize>::new();
    for (role, group) in FilterGroup::ALL.iter().enumerate().skip(1) {
        let presets = prepared.for_role(role);
        let mut in_row = vec![false; all.len()];
        for preset in presets {
            if preset.pattern.is_none() || preset.is_favorite || preset.is_drum_kit {
                continue;
            }
            per_row.insert(
                format!("{} › {}", group.label(), preset.label),
                preset.matches.len(),
            );
            for &index in preset.matches.iter() {
                in_row[index] = true;
            }
        }
        without_row.insert(
            group.label(),
            presets[0]
                .matches
                .iter()
                .filter(|&&index| !in_row[index])
                .count(),
        );
    }
    println!("{} patches", all.len());
    for (row, count) in &per_row {
        println!("row {row}: {count}");
    }
    for (role, count) in &without_row {
        println!("no builtin row in {role}: {count}");
    }
    for (rows, count) in &overlaps {
        println!("overlap {rows}: {count}");
    }
    println!("overlapping patches: {}", overlaps.values().sum::<usize>());
    assert!(overlaps.is_empty());
}
