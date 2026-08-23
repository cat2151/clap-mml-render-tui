use super::*;

use cmrt_runtime::{
    PatchRoles, DEXED_PLUGIN_ID, FLOE_PLUGIN_ID, SFORZANDO_PLUGIN_ID, SURGE_XT_PLUGIN_ID,
    VAPORIZER2_PLUGIN_ID,
};

fn plugin(name: &str, plugin_id: &str) -> CatalogPlugin {
    CatalogPlugin {
        name: name.to_string(),
        plugin_path: format!("/clap/{name}.clap"),
        plugin_id: Some(plugin_id.to_string()),
        base: None,
        dirs: Vec::new(),
        resolved_patches: None,
        source_notices: Vec::new(),
        patch_roles: PatchRoles::default(),
    }
}

fn routed_name<'a>(plugins: &'a PatchPlugins, patch: &str) -> &'a str {
    &plugins.for_patch(patch).unwrap().name
}

#[test]
fn a_single_concrete_catalog_rejects_an_unsupported_patch() {
    let plugins = PatchPlugins::new(vec![plugin("Surge XT", SURGE_XT_PLUGIN_ID)]);

    assert_eq!(routed_name(&plugins, "Pads/Pad 1.fxp"), "Surge XT");
    assert!(matches!(
        plugins.for_patch("Dexed_01.syx/00 Bell"),
        Err(cmrt_core::RouteError::Unsupported { .. })
    ));
    assert!(plugins.any_external_voicing());
}

#[test]
fn a_mixed_catalog_routes_each_patch_form_to_its_own_plugin() {
    let plugins = PatchPlugins::new(vec![
        plugin("Surge XT", SURGE_XT_PLUGIN_ID),
        plugin("Dexed", DEXED_PLUGIN_ID),
    ]);

    assert_eq!(routed_name(&plugins, "Pads/Pad 1.fxp"), "Surge XT");
    assert_eq!(routed_name(&plugins, "Dexed_01.syx/00 Bell"), "Dexed");
}

/// `.vvp` は「1 ファイル = 1 音色」で `.fxp` と単位が同じだが、**Surge の添字へ落として
/// はいけない**。落とすと Surge のカテゴリ（`Pads` / `Basses` …）で絞られ、Vaporizer2 の
/// 展開名（`Pad` / `Bass`）と綴りが合わずに chord / bass / arpeggio 行の候補から全部消える。
#[test]
fn a_vvp_patch_goes_to_vaporizer2_not_to_the_other_state_file_plugin() {
    let plugins = PatchPlugins::new(vec![
        plugin("Surge XT", SURGE_XT_PLUGIN_ID),
        plugin("Dexed", DEXED_PLUGIN_ID),
        plugin("Vaporizer2", VAPORIZER2_PLUGIN_ID),
    ]);

    assert_eq!(routed_name(&plugins, "AR Accent Arp.vvp"), "Vaporizer2");
    assert_eq!(routed_name(&plugins, "MyBank/PD Emily.VVP"), "Vaporizer2");
    assert_eq!(routed_name(&plugins, "Pads/Pad 1.fxp"), "Surge XT");
    assert_eq!(routed_name(&plugins, "Dexed_01.syx/00 Bell"), "Dexed");
}

#[test]
fn an_unsupported_patch_does_not_fall_back_to_the_first_plugin() {
    let plugins = PatchPlugins::new(vec![
        plugin("Dexed", DEXED_PLUGIN_ID),
        plugin("Surge XT", SURGE_XT_PLUGIN_ID),
    ]);

    assert!(matches!(
        plugins.for_patch("AR Accent Arp.vvp"),
        Err(cmrt_core::RouteError::Unsupported { .. })
    ));
}

/// 既定プラグイン（先頭）が優先される。Dexed が既定なら cartridge は Dexed のまま。
#[test]
fn the_default_plugin_wins_when_it_handles_the_form() {
    let plugins = PatchPlugins::new(vec![
        plugin("Dexed", DEXED_PLUGIN_ID),
        plugin("Surge XT", SURGE_XT_PLUGIN_ID),
    ]);

    assert_eq!(routed_name(&plugins, "Dexed_01.syx/00 Bell"), "Dexed");
    assert_eq!(routed_name(&plugins, "Pads/Pad 1.fxp"), "Surge XT");
}

/// Surge を 1 つも積んでいなければ共有 voicing JSON は取りに行かない。
#[test]
fn a_catalog_without_surge_reports_no_surge() {
    let plugins = PatchPlugins::new(vec![plugin("Dexed", DEXED_PLUGIN_ID)]);

    assert!(!plugins.any_external_voicing());
}

#[test]
fn four_plugin_catalog_routes_floe_only_to_floe() {
    let plugins = PatchPlugins::new(vec![
        plugin("Surge XT", SURGE_XT_PLUGIN_ID),
        plugin("Dexed", DEXED_PLUGIN_ID),
        plugin("Vaporizer2", VAPORIZER2_PLUGIN_ID),
        plugin("Floe", FLOE_PLUGIN_ID),
    ]);

    assert_eq!(routed_name(&plugins, "Pads/Pad 1.fxp"), "Surge XT");
    assert_eq!(routed_name(&plugins, "Dexed.syx/00 Bell"), "Dexed");
    assert_eq!(routed_name(&plugins, "PD Emily.vvp"), "Vaporizer2");
    assert_eq!(
        routed_name(&plugins, "Celtic Harp/Realistic.floe-preset"),
        "Floe"
    );
}

#[test]
fn five_plugin_catalog_routes_sfz_only_to_sforzando() {
    let plugins = PatchPlugins::new(vec![
        plugin("Surge XT", SURGE_XT_PLUGIN_ID),
        plugin("Dexed", DEXED_PLUGIN_ID),
        plugin("Vaporizer2", VAPORIZER2_PLUGIN_ID),
        plugin("Floe", FLOE_PLUGIN_ID),
        plugin("Sforzando", SFORZANDO_PLUGIN_ID),
    ]);

    assert_eq!(
        routed_name(&plugins, "Garritan/Glockenspiel.sfz"),
        "Sforzando"
    );
    assert_eq!(
        routed_name(&plugins, "Garritan/Glockenspiel.SFZ"),
        "Sforzando"
    );
    assert_eq!(routed_name(&plugins, "Pads/Pad 1.fxp"), "Surge XT");
    assert_eq!(routed_name(&plugins, "Dexed.syx/00 Bell"), "Dexed");
    assert_eq!(routed_name(&plugins, "PD Emily.vvp"), "Vaporizer2");
    assert_eq!(
        routed_name(&plugins, "Celtic Harp/Realistic.floe-preset"),
        "Floe"
    );
}

#[test]
fn a_missing_plugin_is_reported_instead_of_falling_back() {
    let plugins = PatchPlugins::new(vec![plugin("Dexed", DEXED_PLUGIN_ID)]);

    assert!(matches!(
        plugins.for_patch("Celtic Harp/Realistic.floe-preset"),
        Err(cmrt_core::RouteError::Unsupported { .. })
    ));
}
