use super::*;

#[test]
fn server_selector_metadata_reaches_the_overlay_entry() {
    let plugin = cmrt_tui_core::patch_plugins::CatalogPlugin {
        name: "Surge XT".to_string(),
        plugin_path: "C:/Surge XT.clap".to_string(),
        plugin_id: Some(cmrt_runtime::SURGE_XT_PLUGIN_ID.to_string()),
        base: Some("C:/patches".to_string()),
        dirs: Vec::new(),
        resolved_patches: None,
        source_notices: Vec::new(),
    };
    let info = cmrt_core::AudioPluginInfo::new(
        plugin.name.clone(),
        plugin.plugin_path.clone(),
        plugin.plugin_id.clone(),
        plugin.base.clone(),
    );
    let audio = info.describe_patch("patches_factory/Basses/Attacky.fxp", None);
    let snapshot = cmrt_tui_core::patch_load::PatchCatalogSnapshot::new(
        vec![(
            audio.reference.display.clone(),
            audio.normalized_display.clone(),
        )],
        vec![audio],
        vec![plugin],
        Vec::new(),
        BTreeMap::new(),
    );

    let entries = mml_overlay_catalog_entries(&snapshot);

    assert_eq!(entries[0].plugin_sort_key(), "surge xt");
    assert_eq!(entries[0].selector_category(), Some("Basses"));
}
