use super::*;

use cmrt_runtime::{
    PatchRoles, DEXED_PLUGIN_ID, FLOE_PLUGIN_ID, SURGE_XT_PLUGIN_ID, VAPORIZER2_PLUGIN_ID,
};

fn plugin(name: &str, plugin_id: &str) -> CatalogPlugin {
    CatalogPlugin {
        name: name.to_string(),
        plugin_path: format!("/clap/{name}.clap"),
        plugin_id: Some(plugin_id.to_string()),
        base: None,
        dirs: Vec::new(),
        patch_roles: PatchRoles::default(),
    }
}

/// プラグインが 1 つだけのカタログでは、どの形の patch も同じプラグインへ落ちる。
/// カタログに `.syx` が並ばない今日の構成では、この経路しか通らない。
#[test]
fn a_single_plugin_catalog_answers_the_same_for_every_patch_form() {
    let plugins = PatchPlugins::new(vec![plugin("Surge XT", SURGE_XT_PLUGIN_ID)]);

    assert_eq!(plugins.for_patch("Pads/Pad 1.fxp").name, "Surge XT");
    assert_eq!(plugins.for_patch("Dexed_01.syx/00 Bell").name, "Surge XT");
    assert!(plugins.any_surge_xt());
}

#[test]
fn a_mixed_catalog_routes_each_patch_form_to_its_own_plugin() {
    let plugins = PatchPlugins::new(vec![
        plugin("Surge XT", SURGE_XT_PLUGIN_ID),
        plugin("Dexed", DEXED_PLUGIN_ID),
    ]);

    assert_eq!(plugins.for_patch("Pads/Pad 1.fxp").name, "Surge XT");
    assert_eq!(plugins.for_patch("Dexed_01.syx/00 Bell").name, "Dexed");
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

    assert_eq!(plugins.for_patch("AR Accent Arp.vvp").name, "Vaporizer2");
    assert_eq!(plugins.for_patch("MyBank/PD Emily.VVP").name, "Vaporizer2");
    assert_eq!(plugins.for_patch("Pads/Pad 1.fxp").name, "Surge XT");
    assert_eq!(plugins.for_patch("Dexed_01.syx/00 Bell").name, "Dexed");
}

/// Vaporizer2 がカタログに無ければ `.vvp` は既定プラグインへ落ちる。候補が減るだけで
/// 済ませる既存の倒れ方（`PatchPlugins::new` の doc）を `.vvp` でも変えない。
#[test]
fn a_vvp_patch_falls_back_to_the_default_plugin_when_vaporizer2_is_absent() {
    let plugins = PatchPlugins::new(vec![
        plugin("Dexed", DEXED_PLUGIN_ID),
        plugin("Surge XT", SURGE_XT_PLUGIN_ID),
    ]);

    assert_eq!(plugins.for_patch("AR Accent Arp.vvp").name, "Dexed");
}

/// 既定プラグイン（先頭）が優先される。Dexed が既定なら cartridge は Dexed のまま。
#[test]
fn the_default_plugin_wins_when_it_handles_the_form() {
    let plugins = PatchPlugins::new(vec![
        plugin("Dexed", DEXED_PLUGIN_ID),
        plugin("Surge XT", SURGE_XT_PLUGIN_ID),
    ]);

    assert_eq!(plugins.for_patch("Dexed_01.syx/00 Bell").name, "Dexed");
    assert_eq!(plugins.for_patch("Pads/Pad 1.fxp").name, "Surge XT");
}

/// Surge を 1 つも積んでいなければ共有 voicing JSON は取りに行かない。
#[test]
fn a_catalog_without_surge_reports_no_surge() {
    let plugins = PatchPlugins::new(vec![plugin("Dexed", DEXED_PLUGIN_ID)]);

    assert!(!plugins.any_surge_xt());
}

#[test]
fn four_plugin_catalog_routes_floe_only_to_floe() {
    let plugins = PatchPlugins::new(vec![
        plugin("Surge XT", SURGE_XT_PLUGIN_ID),
        plugin("Dexed", DEXED_PLUGIN_ID),
        plugin("Vaporizer2", VAPORIZER2_PLUGIN_ID),
        plugin("Floe", FLOE_PLUGIN_ID),
    ]);

    assert_eq!(plugins.for_patch("Pads/Pad 1.fxp").name, "Surge XT");
    assert_eq!(plugins.for_patch("Dexed.syx/00 Bell").name, "Dexed");
    assert_eq!(plugins.for_patch("PD Emily.vvp").name, "Vaporizer2");
    assert_eq!(
        plugins.for_patch("Celtic Harp/Realistic.floe-preset").name,
        "Floe"
    );
}

#[test]
fn floe_absence_keeps_the_existing_fallback() {
    let plugins = PatchPlugins::new(vec![plugin("Dexed", DEXED_PLUGIN_ID)]);

    assert_eq!(
        plugins.for_patch("Celtic Harp/Realistic.floe-preset").name,
        "Dexed"
    );
}
