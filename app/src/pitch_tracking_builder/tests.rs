use super::*;

const SURGE_ID: &str = "org.surge-synth-team.surge-xt";
const DEXED_ID: &str = "com.digital-suburban.dexed";
const VAST_ID: &str = "com.vastdynamics.VAST2";

struct Owned {
    display: &'static str,
    normalized: String,
    category: Option<&'static str>,
    plugin: (&'static str, &'static str),
}

fn owned(
    display: &'static str,
    category: Option<&'static str>,
    plugin: (&'static str, &'static str),
) -> Owned {
    Owned {
        display,
        normalized: display.to_lowercase(),
        category,
        plugin,
    }
}

fn targets(entries: &[Owned]) -> Vec<String> {
    let inputs = entries
        .iter()
        .map(|entry| TargetInput {
            display: entry.display,
            normalized_display: &entry.normalized,
            selector_category: entry.category,
            plugin_name: Some(entry.plugin.0),
            plugin_id: Some(entry.plugin.1),
        })
        .collect::<Vec<_>>();
    percussion_targets(&inputs)
        .into_iter()
        .map(|target| target.display)
        .collect()
}

const SURGE: (&str, &str) = ("Surge XT", SURGE_ID);
const DEXED: (&str, &str) = ("Dexed", DEXED_ID);
const VAST: (&str, &str) = ("Vaporizer2", VAST_ID);

#[test]
fn kick_snare_and_hihat_rows_are_not_targets() {
    let entries = [
        owned(
            "patches_factory/Percussion/Kick 909ish.fxp",
            Some("Percussion"),
            SURGE,
        ),
        owned(
            "patches_factory/Percussion/Snare Tight.fxp",
            Some("Percussion"),
            SURGE,
        ),
        owned(
            "patches_factory/Percussion/Closed Hat.fxp",
            Some("Percussion"),
            SURGE,
        ),
        owned(
            "patches_factory/Percussion/Drum One.fxp",
            Some("Percussion"),
            SURGE,
        ),
    ];
    assert_eq!(
        targets(&entries),
        ["patches_factory/Percussion/Drum One.fxp"]
    );
}

#[test]
fn surge_patch_with_percussion_category_is_a_target_without_a_drum_word_in_its_name() {
    let entries = [owned(
        "patches_3rdparty/Someone/Glass Ping.fxp",
        Some("Percussion"),
        SURGE,
    )];
    assert_eq!(
        targets(&entries),
        ["patches_3rdparty/Someone/Glass Ping.fxp"]
    );
}

#[test]
fn percussion_of_every_plugin_is_a_target_and_melodic_patches_are_not() {
    let entries = [
        owned(
            "Dexed_cart_1.0/!Instruments/Keyboard/Synth/Analog/ANALOG1.SYX/09 PERCSYN 1",
            None,
            DEXED,
        ),
        owned("Conga Hit.vvp", None, VAST),
        owned("Saw Lead.vvp", None, VAST),
    ];
    assert_eq!(
        targets(&entries),
        [
            "Dexed_cart_1.0/!Instruments/Keyboard/Synth/Analog/ANALOG1.SYX/09 PERCSYN 1",
            "Conga Hit.vvp",
        ]
    );
}

#[test]
fn target_keeps_the_plugin_id_of_its_catalog_entry() {
    let normalized = "conga hit.vvp".to_string();
    let inputs = [TargetInput {
        display: "Conga Hit.vvp",
        normalized_display: &normalized,
        selector_category: None,
        plugin_name: Some("Vaporizer2"),
        plugin_id: Some(VAST_ID),
    }];
    assert_eq!(
        percussion_targets(&inputs),
        [Target {
            display: "Conga Hit.vvp".into(),
            plugin_id: Some(VAST_ID.into()),
        }]
    );
}

#[test]
fn probe_stdout_is_read_after_plugin_log_lines() {
    // server と同じ整形（内側の object も `{` だけの行になる）で出す。
    let json = serde_json::to_string_pretty(&serde_json::json!({
        "patch": "Drums/Hihat Closed.sxsnp",
        "tracks_keyboard": false,
        "observations": [
            { "note": 48, "frequency_hz": null },
            { "note": 60, "frequency_hz": 261.5 },
        ],
    }))
    .unwrap();
    let stdout = format!("src/clap/entry.cpp:190 Initializing\n{{\nnot json\n{json}\n");
    assert_eq!(
        parse_probe_stdout(&stdout).unwrap(),
        ProbeReport {
            tracks_keyboard: false,
            observations: vec![
                NoteFrequency {
                    note: 48,
                    frequency_hz: None,
                },
                NoteFrequency {
                    note: 60,
                    frequency_hz: Some(261.5),
                },
            ],
        }
    );
}

#[test]
fn probe_stdout_without_json_is_an_error() {
    assert!(parse_probe_stdout("plugin log only\n").is_err());
}

#[test]
fn written_file_has_the_shared_schema() {
    let dir = std::env::temp_dir().join(format!("cmrt-pitch-tracking-{}", std::process::id()));
    let path = dir.join("patch-pitch-tracking.json");
    let file = PitchTrackingFile {
        format_version: FORMAT_VERSION,
        entries: vec![PitchTrackingEntry {
            plugin_id: VAST_ID.into(),
            display: "Conga Hit.vvp".into(),
            tracks_keyboard: true,
            observations: vec![NoteFrequency {
                note: 48,
                frequency_hz: Some(130.8),
            }],
        }],
    };
    write_file(&path, &file).unwrap();
    let value: serde_json::Value = serde_json::from_slice(&std::fs::read(&path).unwrap()).unwrap();
    let _ = std::fs::remove_dir_all(&dir);
    assert_eq!(
        value,
        serde_json::json!({
            "format_version": 1,
            "entries": [{
                "plugin_id": VAST_ID,
                "display": "Conga Hit.vvp",
                "tracks_keyboard": true,
                "observations": [{ "note": 48, "frequency_hz": 130.8 }],
            }],
        })
    );
}
