use std::{collections::HashSet, sync::Arc};

use cmrt_patches::PatchRole;
use cmrt_tui_core::patch_load::PatchCatalogSnapshot as HostSnapshot;

use super::*;

const LEADS: [&str; 2] = ["Leads/Lead 1.fxp", "Leads/Lead 2.fxp"];

fn pads() -> Vec<String> {
    (0..12)
        .map(|index| format!("Pads/Pad {index:02}.fxp"))
        .collect()
}

fn names() -> Vec<String> {
    LEADS
        .iter()
        .map(|name| name.to_string())
        .chain(pads())
        .collect()
}

fn snapshot(names: &[String], user_presets: &[(String, String)]) -> HostSnapshot {
    let pairs = names
        .iter()
        .map(|name| (name.clone(), name.to_lowercase()))
        .collect();
    let mut snapshot = HostSnapshot::from_pairs(pairs);
    snapshot.rebuild_patch_roles(user_presets);
    snapshot
}

fn load(catalog: &mut KeyboardPatchCatalog, names: &[String], current: Option<&str>) {
    let snapshot = snapshot(names, &[]);
    let state = PatchLoadState::Ready(Arc::new(snapshot));
    let PatchLoadState::Ready(snapshot) = &state else {
        unreachable!()
    };
    catalog.load(host_patch_catalog(&state), snapshot.role_presets(), current);
}

fn loaded(current: Option<&str>) -> KeyboardPatchCatalog {
    let mut catalog = KeyboardPatchCatalog::default();
    load(&mut catalog, &names(), current);
    catalog
}

fn role_index(role: PatchRole) -> usize {
    FilterGroup::ALL
        .iter()
        .position(|group| group.role() == Some(role))
        .unwrap()
}

fn preset_index(catalog: &KeyboardPatchCatalog, label: &str) -> usize {
    catalog
        .presets()
        .iter()
        .position(|preset| preset.label == label)
        .unwrap_or_else(|| panic!("preset {label:?} is listed"))
}

fn focus_role(catalog: &mut KeyboardPatchCatalog) {
    catalog.move_focus(-2);
    assert_eq!(catalog.focus(), PatchPaneFocus::Role);
}

#[test]
fn load_selects_the_current_patch_without_changing_unknown_patch() {
    let catalog = loaded(Some("Leads/Lead 2.fxp"));
    assert_eq!(catalog.selected_patch_index(), Some(1));
    assert_eq!(catalog.selected_patch(), Some("Leads/Lead 2.fxp"));
    assert_eq!(catalog.focus(), PatchPaneFocus::Patches);
    assert_eq!((catalog.role_cursor(), catalog.preset_cursor()), (0, 0));

    let catalog = loaded(Some("Unknown"));
    assert_eq!(catalog.selected_patch_index(), None);
    assert_eq!(catalog.selected_patch(), None);
}

#[test]
fn patch_navigation_clamps_and_moves_by_ten() {
    let mut catalog = loaded(Some("Pads/Pad 00.fxp"));

    assert_eq!(
        catalog.move_focused_cursor(10).as_deref(),
        Some("Pads/Pad 10.fxp")
    );
    assert_eq!(
        catalog.move_focused_cursor(10).as_deref(),
        Some("Pads/Pad 11.fxp")
    );
    assert_eq!(catalog.move_focused_cursor(1), None);
    assert_eq!(
        catalog.move_focused_cursor(-10).as_deref(),
        Some("Pads/Pad 01.fxp")
    );
    assert_eq!(
        catalog.move_focused_to_start().as_deref(),
        Some("Leads/Lead 1.fxp")
    );
    assert_eq!(
        catalog.move_focused_to_end().as_deref(),
        Some("Pads/Pad 11.fxp")
    );
}

#[test]
fn first_navigation_from_an_unknown_patch_selects_the_first_patch() {
    let mut catalog = loaded(Some("Unknown"));

    assert_eq!(
        catalog.move_focused_cursor(-1).as_deref(),
        Some("Leads/Lead 1.fxp")
    );
    assert_eq!(catalog.selected_patch_index(), Some(0));
}

#[test]
fn focus_moves_between_the_three_panes_and_stops_at_both_ends() {
    let mut catalog = loaded(Some("Leads/Lead 1.fxp"));
    assert_eq!(catalog.focus(), PatchPaneFocus::Patches);

    catalog.move_focus(1);
    assert_eq!(catalog.focus(), PatchPaneFocus::Patches);
    catalog.move_focus(-1);
    assert_eq!(catalog.focus(), PatchPaneFocus::Preset);
    catalog.move_focus(-1);
    assert_eq!(catalog.focus(), PatchPaneFocus::Role);
    catalog.move_focus(-1);
    assert_eq!(catalog.focus(), PatchPaneFocus::Role);
    catalog.move_focus(1);
    catalog.move_focus(1);
    assert_eq!(catalog.focus(), PatchPaneFocus::Patches);
    assert_eq!(catalog.selected_patch(), Some("Leads/Lead 1.fxp"));
}

#[test]
fn moving_the_role_resets_the_preset_and_keeps_the_patch_when_it_is_listed() {
    let mut catalog = loaded(Some("Pads/Pad 03.fxp"));
    catalog.move_focus(-1);
    let pad = preset_index(&catalog, "Chord › pad");
    assert_eq!(catalog.move_focused_cursor(pad as isize), None);
    assert_eq!(catalog.preset_cursor(), pad);
    focus_role(&mut catalog);

    // Pad は Chord track。現在の音色が残るので音色は変わらない。
    let chord = role_index(PatchRole::Chord);
    assert_eq!(catalog.move_focused_cursor(chord as isize), None);
    assert_eq!(catalog.role_cursor(), chord);
    assert_eq!(catalog.preset_cursor(), 0);
    assert_eq!(catalog.selected_patch(), Some("Pads/Pad 03.fxp"));
    assert_eq!(catalog.patches().len(), 12);

    // Lead / melody には Pad が無いので先頭の Lead へ移る。
    assert_eq!(
        catalog.move_focused_cursor(1).as_deref(),
        Some("Leads/Lead 1.fxp")
    );
    assert_eq!(catalog.role_cursor(), role_index(PatchRole::Lead));
    assert_eq!(catalog.patches().len(), 2);

    // 端で止まる。
    assert_eq!(catalog.move_focused_to_end(), None);
    assert_eq!(catalog.role_cursor(), FilterGroup::ALL.len() - 1);
    assert_eq!(catalog.patches().len(), 0);
    assert_eq!(catalog.selected_patch(), None);
    assert_eq!(catalog.move_focused_cursor(1), None);
}

#[test]
fn moving_the_preset_keeps_the_patch_when_it_is_listed_or_jumps_to_the_first() {
    let mut catalog = loaded(Some("Pads/Pad 03.fxp"));
    catalog.move_focus(-1);
    assert_eq!(catalog.focus(), PatchPaneFocus::Preset);

    let pad = preset_index(&catalog, "Chord › pad");
    assert_eq!(catalog.move_focused_cursor(pad as isize), None);
    assert_eq!(catalog.selected_patch(), Some("Pads/Pad 03.fxp"));

    let lead = preset_index(&catalog, "Lead › lead");
    assert_eq!(
        catalog
            .move_focused_cursor(lead as isize - pad as isize)
            .as_deref(),
        Some("Leads/Lead 1.fxp")
    );
    assert_eq!(catalog.preset_cursor(), lead);
    assert_eq!(catalog.move_focused_cursor(-(lead as isize)), None);
    assert_eq!(catalog.preset_cursor(), 0);
}

#[test]
fn random_selection_visits_every_other_patch_without_duplicates() {
    let mut catalog = loaded(Some("Leads/Lead 1.fxp"));
    let mut seen = HashSet::new();

    for _ in 0..13 {
        let patch = catalog
            .select_random_patch()
            .expect("another patch should be available");
        assert_ne!(patch, "Leads/Lead 1.fxp");
        assert!(seen.insert(patch), "random cycle returned a duplicate");
    }

    assert_eq!(seen.len(), 13);
    let previous = catalog.selected_patch().map(str::to_string);
    assert_ne!(catalog.select_random_patch(), previous);

    catalog.select(2);
    assert_ne!(
        catalog.select_random_patch().as_deref(),
        Some("Pads/Pad 00.fxp")
    );
}

#[test]
fn random_selection_stays_inside_the_selected_role_and_focuses_patches() {
    let mut catalog = loaded(Some("Leads/Lead 1.fxp"));
    focus_role(&mut catalog);
    let chord = role_index(PatchRole::Chord);
    assert_eq!(
        catalog.move_focused_cursor(chord as isize).as_deref(),
        Some("Pads/Pad 00.fxp")
    );
    catalog.move_focus(-1);

    for _ in 0..20 {
        let patch = catalog.select_random_patch().unwrap();
        assert!(patch.starts_with("Pads/"), "{patch}");
        assert_eq!(catalog.focus(), PatchPaneFocus::Patches);
    }
}

#[test]
fn random_selection_updates_the_cursor_from_an_unknown_patch() {
    let mut catalog = loaded(Some("Unknown"));

    let patch = catalog.select_random_patch().unwrap();
    assert_eq!(catalog.selected_patch(), Some(patch.as_str()));
}

#[test]
fn random_selection_with_only_the_current_patch_does_nothing() {
    let single = vec!["Leads/Lead 1.fxp".to_string()];
    let mut catalog = KeyboardPatchCatalog::default();
    load(&mut catalog, &single, Some("Leads/Lead 1.fxp"));

    assert_eq!(catalog.select_random_patch(), None);

    load(&mut catalog, &single, Some("Unknown"));
    assert_eq!(
        catalog.select_random_patch().as_deref(),
        Some("Leads/Lead 1.fxp")
    );
}

#[test]
fn role_counts_match_the_shared_role_index() {
    let snapshot = snapshot(&names(), &[]);
    let mut catalog = loaded(Some("Leads/Lead 1.fxp"));
    focus_role(&mut catalog);
    for role in PatchRole::ALL {
        catalog.move_focused_to_start();
        catalog.move_focused_cursor(role_index(role) as isize);
        assert_eq!(
            catalog.patches().len(),
            snapshot.patch_roles().candidates(role).len(),
            "{role:?}"
        );
    }
}

#[test]
fn changed_user_presets_rebuild_the_preset_list_and_keep_the_selection() {
    let mut catalog = loaded(Some("Pads/Pad 03.fxp"));
    focus_role(&mut catalog);
    let chord = role_index(PatchRole::Chord);
    catalog.move_focused_cursor(chord as isize);
    catalog.move_focus(1);
    let pad = preset_index(&catalog, "pad");
    assert_eq!(catalog.move_focused_cursor(pad as isize), None);
    let before = catalog.presets().len();

    let presets = [("chord".to_string(), "sub".to_string())];
    let snapshot = snapshot(&names(), &presets);
    let state = PatchLoadState::Ready(Arc::new(snapshot));
    catalog.reload_presets(
        host_patch_catalog(&state),
        &presets,
        Some("Pads/Pad 03.fxp"),
    );

    assert_eq!(catalog.presets().len(), before + 1);
    assert!(catalog
        .presets()
        .iter()
        .any(|preset| preset.is_user && preset.label == "sub"));
    assert_eq!(catalog.focus(), PatchPaneFocus::Preset);
    assert_eq!(catalog.role_cursor(), chord);
    assert_eq!(catalog.preset_cursor(), pad);
    assert_eq!(catalog.selected_patch(), Some("Pads/Pad 03.fxp"));
}
