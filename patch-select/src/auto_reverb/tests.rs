use super::*;

use cmrt_core::AudioEffectPluginInfo;
use cmrt_patches::PatchRoleInput;

pub(crate) const DEXED_BASS: &str = "Factory.syx/03 DX Bass 1";
pub(crate) const DEXED_KICK: &str = "Drums.syx/00 Kick Hard";
pub(crate) const DEXED_SNARE: &str = "Drums.syx/01 Snare Tight";
pub(crate) const DEXED_PAD: &str = "Factory.syx/10 Warm Pad";
pub(crate) const SURGE_PAD: &str = "Pads/Pad 1.fxp";

/// マシンに依存しない catalog。Dragonfly の Room/Hall と、Surge の reverb 2 つと delay。
pub(crate) fn test_catalog() -> AudioEffectCatalog {
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
    let surge = AudioEffectPluginInfo::new(
        "Surge XT Effects",
        "/clap/surge-fx.clap",
        "org.example.surge-fx",
        "/presets/surge-fx",
    );
    let preset = |plugin: &AudioEffectPluginInfo, value: &str, kind: &str| AudioEffectPreset {
        plugin: plugin.key.clone(),
        json_key: plugin.json_key.clone(),
        value: value.to_string(),
        display: format!("{}: {value}", plugin.name),
        name: value.to_string(),
        category: "Space / Imaging".to_string(),
        kind: kind.to_string(),
        path: std::path::PathBuf::from(format!("/presets/{value}")),
    };
    let presets = vec![
        preset(&room, "Small Drum Room", "Reverb"),
        preset(&room, "Large Drum Room", "Reverb"),
        preset(&hall, "Medium Clear Hall", "Reverb"),
        preset(&surge, "Reverb 1/Hall", "Reverb"),
        preset(&surge, "Reverb 2/Room", "Reverb"),
        preset(&surge, "Delay/Echo", "Delay"),
    ];
    AudioEffectCatalog::with_entries(vec![room, hall, surge], presets)
}

fn role_index() -> PatchRoleIndex {
    let displays = [DEXED_BASS, DEXED_KICK, DEXED_SNARE, DEXED_PAD, SURGE_PAD];
    let lowered: Vec<String> = displays.iter().map(|d| d.to_lowercase()).collect();
    PatchRoleIndex::build(
        displays
            .iter()
            .zip(&lowered)
            .map(|(display, normalized)| PatchRoleInput {
                display,
                normalized_display: normalized,
                selector_category: None,
            }),
        &[],
    )
}

fn resolve_default(display: &str, has_builtin_effects: bool) -> AutoReverb {
    resolve(
        display,
        has_builtin_effects,
        &role_index(),
        Some(&test_catalog()),
        &AutoReverbRules::default(),
    )
}

fn row_index(rules: &AutoReverbRules, name: &str) -> usize {
    rules
        .rows()
        .iter()
        .position(|(row, _)| row.name() == name)
        .unwrap()
}

#[test]
fn default_rules_leave_bass_and_kick_dry() {
    assert_eq!(
        resolve_default(DEXED_BASS, false),
        AutoReverb::Dry {
            row: "bass|bs".to_string()
        }
    );
    assert_eq!(
        resolve_default(DEXED_KICK, false),
        AutoReverb::Dry {
            row: "kick|bass drum".to_string()
        }
    );
}

#[test]
fn default_rules_put_a_drum_room_on_snare_and_a_hall_on_pad() {
    assert_eq!(
        resolve_default(DEXED_SNARE, false),
        AutoReverb::Apply {
            stage: serde_json::json!({"Dragonfly Room Reverb preset": "Small Drum Room"}),
            effect_name: "Dragonfly Room Reverb: Small Drum Room".to_string(),
            row: "snare".to_string(),
        }
    );
    assert_eq!(
        resolve_default(DEXED_PAD, false),
        AutoReverb::Apply {
            stage: serde_json::json!({"Dragonfly Hall Reverb preset": "Medium Clear Hall"}),
            effect_name: "Dragonfly Hall Reverb: Medium Clear Hall".to_string(),
            row: "pad".to_string(),
        }
    );
}

#[test]
fn a_patch_with_builtin_effects_gets_no_auto_reverb() {
    assert_eq!(resolve_default(SURGE_PAD, true), AutoReverb::Builtin);
}

#[test]
fn off_wins_over_every_other_result() {
    let mut rules = AutoReverbRules::default();
    rules.set_enabled(false);
    for (display, builtin) in [(DEXED_SNARE, false), (SURGE_PAD, true)] {
        assert_eq!(
            resolve(
                display,
                builtin,
                &role_index(),
                Some(&test_catalog()),
                &rules
            ),
            AutoReverb::Off
        );
    }
}

#[test]
fn a_missing_catalog_is_reported_as_such() {
    assert_eq!(
        resolve(
            DEXED_SNARE,
            false,
            &role_index(),
            None,
            &AutoReverbRules::default()
        ),
        AutoReverb::NoCatalog
    );
}

#[test]
fn a_saved_value_missing_from_the_catalog_becomes_dry() {
    let saved = BTreeMap::from([(
        "snare".to_string(),
        serde_json::json!({"Dragonfly Room Reverb preset": "No Such Room"}),
    )]);
    let catalog = test_catalog();
    let rules = AutoReverbRules::from_saved(true, &saved, Some(&catalog));

    assert_eq!(rules.to_saved()["snare"], Value::Null);
    assert_eq!(
        resolve(DEXED_SNARE, false, &role_index(), Some(&catalog), &rules),
        AutoReverb::Dry {
            row: "snare".to_string()
        }
    );
}

#[test]
fn a_saved_value_that_is_not_a_selectable_reverb_becomes_dry() {
    let catalog = test_catalog();
    for value in [
        serde_json::json!({"Surge XT Effects preset": "Delay/Echo"}),
        serde_json::json!({"Surge XT Effects preset": "Reverb 1/Hall"}),
    ] {
        let saved = BTreeMap::from([("pad".to_string(), value)]);
        let rules = AutoReverbRules::from_saved(true, &saved, Some(&catalog));
        assert_eq!(rules.to_saved()["pad"], Value::Null);
    }
}

#[test]
fn saved_values_are_kept_when_the_catalog_is_unavailable() {
    let unknown = serde_json::json!({"Dragonfly Room Reverb preset": "No Such Room"});
    let saved = BTreeMap::from([("snare".to_string(), unknown.clone())]);
    let rules = AutoReverbRules::from_saved(true, &saved, None);
    assert_eq!(rules.to_saved()["snare"], unknown);
}

#[test]
fn saved_rules_round_trip() {
    let mut rules = AutoReverbRules::default();
    rules.set_enabled(false);
    let pad = row_index(&rules, "pad");
    rules.set_effect(
        pad,
        Some(serde_json::json!({"Dragonfly Room Reverb preset": "Large Drum Room"})),
    );
    let snare = row_index(&rules, "snare");
    rules.set_effect(snare, None);

    let catalog = test_catalog();
    let restored = AutoReverbRules::from_saved(rules.enabled(), &rules.to_saved(), Some(&catalog));

    assert_eq!(restored, rules);
}

#[test]
fn rows_cover_every_builtin_preset_and_one_other_row_per_role() {
    let rows = auto_reverb_rows();
    assert_eq!(
        rows.len(),
        builtin_role_presets().len() + PatchRole::ALL.len()
    );
    let names: Vec<String> = rows.iter().map(AutoReverbRow::name).collect();
    for name in [
        "bass|bs",
        "snare",
        "fx|effects",
        "bass その他",
        "etc その他",
    ] {
        assert!(names.contains(&name.to_string()), "{name} is missing");
    }
    let mut unique = names.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), names.len());
}

#[test]
fn bass_and_kick_default_dry_drum_rows_the_drum_room_and_the_rest_the_hall() {
    let rules = AutoReverbRules::default();
    let saved = rules.to_saved();
    let room = serde_json::json!({"Dragonfly Room Reverb preset": "Small Drum Room"});
    let hall = serde_json::json!({"Dragonfly Hall Reverb preset": "Medium Clear Hall"});
    for name in ["snare", "hat", "perc", "drum", "drum その他"] {
        assert_eq!(saved[name], room, "{name}");
    }
    for name in ["strings", "lead", "fx|effects", "etc その他"] {
        assert_eq!(saved[name], hall, "{name}");
    }
    for name in ["bass|bs", "bass その他", "kick|bass drum"] {
        assert_eq!(saved[name], Value::Null, "{name}");
    }
    assert!(rules.enabled());
}

#[test]
fn reverb_candidates_are_reverbs_outside_the_excluded_prefix() {
    let catalog = test_catalog();
    assert_eq!(
        reverb_candidates(&catalog)
            .iter()
            .map(|preset| preset.value.as_str())
            .collect::<Vec<_>>(),
        vec![
            "Small Drum Room",
            "Large Drum Room",
            "Medium Clear Hall",
            "Reverb 2/Room"
        ]
    );
}
