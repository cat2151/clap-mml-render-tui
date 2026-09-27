use super::*;

#[test]
fn a_loading_state_becomes_a_loading_catalog() {
    let host = host_patch_catalog(&PatchLoadState::Loading);
    assert_eq!(host.catalog, PatchCatalogSnapshot::Loading);
    assert!(host.load_measurements.is_empty());
}

#[test]
fn an_error_state_carries_its_reason() {
    let host = host_patch_catalog(&PatchLoadState::Err("no patches_dirs".to_string()));
    assert_eq!(
        host.catalog,
        PatchCatalogSnapshot::Error("no patches_dirs".to_string())
    );
}

#[test]
fn a_ready_state_becomes_selector_rows_in_catalog_order() {
    let state = PatchLoadState::ready(vec![
        ("Bass/Deep.fxp".to_string(), "bass/deep.fxp".to_string()),
        ("Lead/Bright.fxp".to_string(), "lead/bright.fxp".to_string()),
    ]);
    let host = host_patch_catalog(&state);
    let PatchCatalogSnapshot::Ready(entries) = host.catalog else {
        panic!("ready state should produce a ready catalog");
    };
    assert_eq!(
        entries
            .iter()
            .map(PatchCatalogEntry::display)
            .collect::<Vec<_>>(),
        vec!["Bass/Deep.fxp", "Lead/Bright.fxp"]
    );
}

#[test]
fn a_ready_state_carries_whether_each_patch_has_builtin_effects() {
    use cmrt_runtime::{DEXED_PLUGIN_ID, SURGE_XT_PLUGIN_ID};
    use cmrt_tui_core::patch_load::PatchCatalogSnapshot as LoadedSnapshot;
    use cmrt_tui_core::patch_plugins::CatalogPlugin;

    let catalog_plugin = |name: &str, plugin_id: &str| CatalogPlugin {
        name: name.to_string(),
        plugin_path: format!("/clap/{name}.clap"),
        plugin_id: Some(plugin_id.to_string()),
        base: None,
        dirs: Vec::new(),
        resolved_patches: None,
        source_notices: Vec::new(),
    };
    let audio_info = |name: &str, plugin_id: &str| {
        cmrt_core::AudioPluginInfo::new(
            name,
            format!("/clap/{name}.clap"),
            Some(plugin_id.to_string()),
            None,
        )
    };
    let surge = audio_info("Surge XT", SURGE_XT_PLUGIN_ID).describe_patch("Pads/Pad 1.fxp", None);
    let dexed = audio_info("Dexed", DEXED_PLUGIN_ID).describe_patch("Factory.syx/00 Init", None);
    let pairs = [&surge, &dexed]
        .iter()
        .map(|patch| {
            (
                patch.reference.display.clone(),
                patch.normalized_display.clone(),
            )
        })
        .collect();
    let snapshot = LoadedSnapshot::new(
        pairs,
        vec![surge, dexed],
        vec![
            catalog_plugin("Surge XT", SURGE_XT_PLUGIN_ID),
            catalog_plugin("Dexed", DEXED_PLUGIN_ID),
        ],
        Vec::new(),
        BTreeMap::new(),
    );

    let host = host_patch_catalog(&PatchLoadState::Ready(std::sync::Arc::new(snapshot)));
    let PatchCatalogSnapshot::Ready(entries) = host.catalog else {
        panic!("ready state should produce a ready catalog");
    };
    assert_eq!(
        entries
            .iter()
            .map(|entry| (entry.display(), entry.has_builtin_effects()))
            .collect::<Vec<_>>(),
        vec![("Pads/Pad 1.fxp", true), ("Factory.syx/00 Init", false)]
    );
}
