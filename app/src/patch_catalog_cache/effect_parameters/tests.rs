use super::*;

#[test]
fn saves_external_layouts_and_preserves_cache_on_invalid_input() {
    let path = std::env::temp_dir().join(format!(
        "cmrt_effect_parameters_{}.json",
        std::process::id()
    ));
    let bytes = serde_json::to_vec(&serde_json::json!({
        "format_version": 2, "generator_version": cmrt_core::surge_fx_preset::PARAM_LAYOUT_GENERATOR_VERSION,
        "plugin_version": "8.7.6", "source_tag": "release_xt_8.7.6",
        "source_revision": "1".repeat(40), "sst_effects_revision": "2".repeat(40),
        "layouts": [{ "fx_type": 71, "display_name": "Invented", "posy_offset": vec![0;12],
            "active": vec![false;12], "names": vec!["";12] }]
    })).unwrap();
    save(&path, &bytes).unwrap();
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert!(save(&path, br#"{"format_version":99}"#).is_err());
    assert_eq!(fs::read(&path).unwrap(), bytes);
    assert_eq!(
        cmrt_core::surge_fx_preset::read_param_layouts(&path)
            .unwrap()
            .len(),
        1
    );
    fs::remove_file(path).unwrap();
}

/// Read the actual files through the same readers used after application startup.
#[test]
#[ignore = "requires CMRT_TEST_SURGE_APP_DIR and an installed Surge XT Effects"]
fn installed_catalog_and_parameter_cache_restore() {
    let app = std::path::PathBuf::from(std::env::var("CMRT_TEST_SURGE_APP_DIR").unwrap());
    super::super::load_from(&app.join("patch-catalog/catalog.json")).unwrap();
    let layouts =
        cmrt_core::surge_fx_preset::read_param_layouts(&app.join(PARAM_LAYOUT_CACHE_RELATIVE_PATH))
            .unwrap();
    assert!(!layouts.is_empty());
    let catalog = cmrt_core::AudioEffectCatalog::discover();
    assert!(catalog
        .presets()
        .iter()
        .any(|preset| preset.plugin.as_str().contains(SURGE_FX_PLUGIN_ID)));
    assert!(!catalog
        .skipped()
        .iter()
        .any(|line| line.contains("Surge XT Effects")));
}
