use super::*;

const DEXED: &str = "com.digital-suburban.dexed";
const SURGE: &str = "org.surge-synth-team.surge-xt";

fn table(entries: &[(&str, &str, bool)]) -> PitchTrackingTable {
    let entries: Vec<_> = entries
        .iter()
        .map(|(plugin_id, display, tracks_keyboard)| {
            serde_json::json!({
                "plugin_id": plugin_id,
                "display": display,
                "tracks_keyboard": tracks_keyboard,
                "observations": [{ "note": 48, "frequency_hz": null }],
            })
        })
        .collect();
    let json = serde_json::json!({ "format_version": 1, "entries": entries });
    PitchTrackingTable::parse(json.to_string().as_bytes()).unwrap()
}

fn candidate<'a>(plugin_id: &'a str, display: &'a str) -> TrackingCandidate<'a, &'a str> {
    TrackingCandidate {
        group: plugin_id,
        plugin_id: Some(plugin_id),
        role: PatchRoleInput {
            display,
            normalized_display: display,
            selector_category: None,
            plugin: None,
        },
    }
}

#[test]
fn only_tracking_percussion_row_patches_are_selected() {
    let candidates = [
        candidate(DEXED, "bank.syx/01 percsyn"),
        candidate(SURGE, "percussion/wood"),
        candidate(DEXED, "bank.syx/02 conga perc"),
        candidate(DEXED, "bank.syx/03 kick"),
        candidate(DEXED, "bank.syx/04 snare"),
        candidate(DEXED, "bank.syx/05 hat"),
        candidate(DEXED, "bank.syx/06 plain lead"),
    ];
    let table = table(&[
        (DEXED, "bank.syx/01 percsyn", true),
        (SURGE, "percussion/wood", true),
        (DEXED, "bank.syx/02 conga perc", false),
        (DEXED, "bank.syx/03 kick", true),
        (DEXED, "bank.syx/04 snare", true),
        (DEXED, "bank.syx/05 hat", true),
        (DEXED, "bank.syx/06 plain lead", true),
    ]);
    assert_eq!(pitched_percussion_indices(&candidates, &table), [0, 1]);
}

#[test]
fn a_display_tracked_for_one_plugin_does_not_move_another_plugins_patch() {
    let candidates = [DEXED, SURGE].map(|plugin_id| candidate(plugin_id, "Percussion/Wood"));
    let table = table(&[(SURGE, "Percussion/Wood", true)]);
    assert_eq!(pitched_percussion_indices(&candidates, &table), [1]);
}

#[test]
fn unknown_format_version_is_rejected() {
    let json = r#"{ "format_version": 2, "entries": [] }"#;
    assert!(PitchTrackingTable::parse(json.as_bytes()).is_err());
}
