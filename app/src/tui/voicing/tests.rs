use super::*;
use crate::voicing_sources::VoicingSourceRefresh;

/// カタログは開発機のインストール状況で変わるので、テストは `Config` を通さず
/// カタログを手で並べる。
fn state(plugins: &[CatalogPlugin]) -> VoicingState {
    VoicingState::new(
        VoicingCache::default(),
        VoicingLayers::default(),
        VoicingSourceRefresh::disabled(),
        VoicingPolicies {
            plugins: PatchPlugins::from_catalog(plugins.to_vec()),
        },
    )
}

fn catalog_plugin(plugin_id: &str, plugin_path: &str) -> CatalogPlugin {
    CatalogPlugin {
        name: String::new(),
        plugin_path: plugin_path.to_string(),
        plugin_id: Some(plugin_id.to_string()),
        base: None,
        dirs: Vec::new(),
        patch_roles: cmrt_runtime::PatchRoles::default(),
    }
}

fn surge_plugin() -> CatalogPlugin {
    catalog_plugin(
        cmrt_runtime::SURGE_XT_PLUGIN_ID,
        cmrt_runtime::default_plugin_path(),
    )
}

fn dexed_plugin() -> CatalogPlugin {
    catalog_plugin(
        cmrt_runtime::DEXED_PLUGIN_ID,
        cmrt_runtime::default_dexed_plugin_path(),
    )
}

#[test]
fn surge_leaves_unknown_patches_undecided() {
    assert_eq!(state(&[surge_plugin()]).resolve("Keys/Unknown.fxp"), None);
}

#[test]
fn plugins_without_patch_level_data_are_all_poly() {
    let state = state(&[dexed_plugin()]);
    assert_eq!(
        state.resolve("SynprezFM/SynprezFM_01.syx/00 Say Again."),
        Some(PatchVoicing::Poly)
    );
    // 名前を知らない patch でも同じ。判定していないのではなく、poly と決めている。
    assert_eq!(state.resolve(""), Some(PatchVoicing::Poly));
}

/// カタログにプラグインが 1 つだけなら、判定方針は全 patch で同じ。
/// `.syx` 形式の patch 文字列を渡しても既定プラグインの方針へ落ちる。
#[test]
fn a_single_plugin_catalog_uses_one_policy_for_every_patch() {
    let surge = state(&[surge_plugin()]);

    assert_eq!(surge.resolve("Keys/Unknown.fxp"), None);
    assert_eq!(surge.resolve("Dexed_01.syx/00 Bell"), None);
}

/// 混在カタログでは方針が patch ごとに変わる。`.fxp` は Surge の層から引き（未判定なら
/// `None`）、cartridge は poly と決める。
#[test]
fn a_mixed_catalog_switches_policy_per_patch() {
    let mixed = state(&[surge_plugin(), dexed_plugin()]);

    assert_eq!(mixed.resolve("Keys/Unknown.fxp"), None);
    assert_eq!(
        mixed.resolve("Dexed_01.syx/00 Bell"),
        Some(PatchVoicing::Poly)
    );
    // 音色を無指定にした行が鳴るのは既定プラグイン（先頭）。
    assert_eq!(mixed.resolve(""), None);
}
