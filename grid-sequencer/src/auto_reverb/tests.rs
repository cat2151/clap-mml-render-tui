use std::collections::BTreeMap;

use cmrt_core::{AudioEffectCatalog, AudioEffectPluginInfo, AudioEffectPreset, AudioPluginInfo};
use cmrt_runtime::{DEXED_PLUGIN_ID, SURGE_XT_PLUGIN_ID};
use cmrt_tui_core::patch_plugins::CatalogPlugin;

use super::*;

pub(crate) const DEXED_SNARE: &str = "Drums.syx/01 Snare Tight";
pub(crate) const DEXED_PAD: &str = "Factory.syx/10 Warm Pad";
pub(crate) const SURGE_PAD: &str = "Pads/Pad 1.fxp";
pub(crate) const DRUM_ROOM_CHAIN: &str = r#"[{"Dragonfly Room Reverb preset":"Small Drum Room"}]"#;
pub(crate) const PAD_HALL_CHAIN: &str = r#"[{"Dragonfly Hall Reverb preset":"Dark Room"}]"#;

/// Dexed（effect 無し）の snare と pad、Surge XT（effect 内蔵）の pad を持つ patch catalog。
pub(crate) fn patch_load() -> PatchLoadState {
    let catalog_plugin = |name: &str, plugin_id: &str| CatalogPlugin {
        name: name.to_string(),
        plugin_path: format!("/clap/{name}.clap"),
        plugin_id: Some(plugin_id.to_string()),
        base: cmrt_runtime::PatchBase::None,
        dirs: Vec::new(),
        resolved_patches: None,
        source_notices: Vec::new(),
    };
    let audio_info = |name: &str, plugin_id: &str| {
        AudioPluginInfo::new(
            name,
            format!("/clap/{name}.clap"),
            Some(plugin_id.to_string()),
            None,
        )
    };
    let dexed = audio_info("Dexed", DEXED_PLUGIN_ID);
    let surge = audio_info("Surge XT", SURGE_XT_PLUGIN_ID);
    let patches = vec![
        dexed.describe_patch(DEXED_SNARE, None),
        dexed.describe_patch(DEXED_PAD, None),
        surge.describe_patch(SURGE_PAD, None),
    ];
    let pairs = patches
        .iter()
        .map(|patch| {
            (
                patch.reference.display.clone(),
                patch.normalized_display.clone(),
            )
        })
        .collect();
    PatchLoadState::Ready(Arc::new(PatchCatalogSnapshot::new(
        pairs,
        patches,
        vec![
            catalog_plugin("Dexed", DEXED_PLUGIN_ID),
            catalog_plugin("Surge XT", SURGE_XT_PLUGIN_ID),
        ],
        Vec::new(),
        BTreeMap::new(),
    )))
}

/// マシンに依存しない effect catalog。既定のルールが使う Dragonfly の 2 つだけ。
pub(crate) fn effect_plugins() -> EffectPlugins {
    let room = AudioEffectPluginInfo::new(
        "Dragonfly Room Reverb",
        "/clap/room.clap",
        "org.example.room",
        "/clap/room.clap",
    );
    let hall = AudioEffectPluginInfo::new(
        "Dragonfly Hall Reverb",
        "/clap/hall.clap",
        "org.example.hall",
        "/clap/hall.clap",
    );
    let preset = |plugin: &AudioEffectPluginInfo, value: &str| AudioEffectPreset {
        plugin: plugin.key.clone(),
        json_key: plugin.json_key.clone(),
        value: value.to_string(),
        display: format!("{}: {value}", plugin.name),
        name: value.to_string(),
        category: "Space / Imaging".to_string(),
        kind: "Reverb".to_string(),
        path: std::path::PathBuf::from(format!("/presets/{value}")),
    };
    let presets = vec![
        preset(&room, "Small Drum Room"),
        preset(&hall, "Medium Clear Hall"),
        preset(&hall, "Dark Room"),
    ];
    EffectPlugins::with_catalog(AudioEffectCatalog::with_entries(vec![room, hall], presets))
}

fn ready_auto_reverb() -> GridAutoReverb {
    let mut auto_reverb = GridAutoReverb::new(effect_plugins());
    auto_reverb.observe(&patch_load());
    auto_reverb
}

fn chain(auto_reverb: &GridAutoReverb, patch: &str) -> String {
    auto_reverb.patch(Some(patch)).effect_chain
}

#[test]
fn a_dexed_snare_gets_the_small_drum_room() {
    assert_eq!(chain(&ready_auto_reverb(), DEXED_SNARE), DRUM_ROOM_CHAIN);
}

#[test]
fn a_dexed_pad_gets_the_pad_hall() {
    assert_eq!(chain(&ready_auto_reverb(), DEXED_PAD), PAD_HALL_CHAIN);
}

#[test]
fn a_surge_patch_has_builtin_effects_and_gets_no_chain() {
    let auto_reverb = ready_auto_reverb();
    assert_eq!(chain(&auto_reverb, SURGE_PAD), "");
    assert_eq!(
        auto_reverb.status(Some(SURGE_PAD)),
        AutoReverbStatus::Resolved(AutoReverb::Builtin)
    );
}

#[test]
fn off_removes_the_chain() {
    let mut auto_reverb = ready_auto_reverb();
    let mut rules = AutoReverbRules::default();
    rules.set_enabled(false);
    auto_reverb.set_rules(rules);

    assert_eq!(chain(&auto_reverb, DEXED_SNARE), "");
    assert_eq!(
        auto_reverb.status(Some(DEXED_SNARE)),
        AutoReverbStatus::Resolved(AutoReverb::Off)
    );
}

#[test]
fn without_a_patch_catalog_nothing_is_added() {
    let mut auto_reverb = GridAutoReverb::new(effect_plugins());
    auto_reverb.observe(&PatchLoadState::Loading);
    assert_eq!(chain(&auto_reverb, DEXED_SNARE), "");
}

#[test]
fn no_patch_has_no_chain() {
    assert_eq!(ready_auto_reverb().patch(None), GridPatch::default());
}
