use super::*;
use crate::voicing_sources::VoicingSourceRefresh;

fn state(policy: VoicingPolicy) -> VoicingState {
    VoicingState::new(
        VoicingCache::default(),
        VoicingLayers::default(),
        VoicingSourceRefresh::disabled(),
        policy,
    )
}

#[test]
fn surge_leaves_unknown_patches_undecided() {
    assert_eq!(
        state(VoicingPolicy::Sources).resolve("Keys/Unknown.fxp"),
        None
    );
}

#[test]
fn plugins_without_patch_level_data_are_all_poly() {
    let state = state(VoicingPolicy::AssumePoly);
    assert_eq!(
        state.resolve("SynprezFM/SynprezFM_01.syx/00 Say Again."),
        Some(PatchVoicing::Poly)
    );
    // 名前を知らない patch でも同じ。判定していないのではなく、poly と決めている。
    assert_eq!(state.resolve(""), Some(PatchVoicing::Poly));
}

#[test]
fn policy_follows_the_active_plugin() {
    let surge = Config {
        plugin_path: cmrt_runtime::default_plugin_path().to_string(),
        ..Config::default()
    };
    assert_eq!(VoicingPolicy::from_config(&surge), VoicingPolicy::Sources);

    let dexed = Config {
        plugin_id: Some(cmrt_runtime::DEXED_PLUGIN_ID.to_string()),
        plugin_path: cmrt_runtime::default_dexed_plugin_path().to_string(),
        ..Config::default()
    };
    assert_eq!(
        VoicingPolicy::from_config(&dexed),
        VoicingPolicy::AssumePoly
    );
}
