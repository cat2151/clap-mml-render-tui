use super::*;

#[test]
fn legacy_rules_use_the_new_vibrato_defaults() {
    let rules =
        RuleTable::from_json(r#"{"columns":{"0":["vibrato"]},"params":{"21":103}}"#).unwrap();
    assert_eq!(
        rules.vibrato_settings(),
        VibratoSettings {
            delay_ms: 300,
            rise_ms: 400,
            depth: 64,
        }
    );
    assert_eq!(rules.param(21), 103);
    assert_eq!(RuleTable::from_json("{}").unwrap(), RuleTable::default());
}

#[test]
fn settings_round_trip_with_speed_stored_only_in_cc21() {
    let mut rules = RuleTable::default();
    let settings = VibratoSettings {
        delay_ms: 650,
        rise_ms: 950,
        depth: 88,
    };
    assert!(rules.set_vibrato_settings(settings));
    assert_eq!(rules.param(21), 95);
    assert!(rules.step_param(21, 8));
    assert_eq!(rules.vibrato_settings(), settings);
    let json = rules.to_json();
    let value: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(value["params"]["21"], 103);
    assert!(value["vibrato"].get("speed").is_none());
    assert_eq!(RuleTable::from_json(&json).unwrap(), rules);
}

#[test]
fn input_and_loaded_settings_stay_in_range_and_report_changes() {
    let oversized = VibratoSettings {
        delay_ms: u16::MAX,
        rise_ms: u16::MAX,
        depth: u8::MAX,
    };
    let bounded = VibratoSettings {
        delay_ms: 5000,
        rise_ms: 5000,
        depth: 127,
    };
    let mut rules = RuleTable::default();
    assert!(!rules.set_vibrato_settings(VibratoSettings::default()));
    assert!(rules.set_vibrato_settings(oversized));
    assert_eq!(rules.vibrato_settings(), bounded);
    assert!(!rules.set_vibrato_settings(oversized));

    let loaded =
        RuleTable::from_json(r#"{"vibrato":{"delay_ms":65535,"rise_ms":65535,"depth":255}}"#)
            .unwrap();
    assert_eq!(loaded.vibrato_settings(), bounded);
    let partial = RuleTable::from_json(r#"{"vibrato":{"delay_ms":0}}"#).unwrap();
    assert_eq!(partial.vibrato_settings().delay_ms, 0);
    assert_eq!(partial.vibrato_settings().rise_ms, 400);
    assert_eq!(partial.vibrato_settings().depth, 64);
}

#[test]
fn clearing_column_rules_keeps_phrase_settings_and_speed() {
    let mut rules = RuleTable::default();
    rules.toggle(0, crate::Rule::Vibrato);
    rules.set_vibrato_settings(VibratoSettings {
        delay_ms: 0,
        rise_ms: 500,
        depth: 0,
    });
    rules.step_param(21, -8);
    let cleared = rules.without_column_rules();
    assert!(cleared.is_empty());
    assert_eq!(cleared.vibrato_settings(), rules.vibrato_settings());
    assert_eq!(cleared.param(21), 87);
}
