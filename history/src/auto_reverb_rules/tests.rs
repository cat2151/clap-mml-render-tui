use super::*;

#[test]
fn settings_round_trip() {
    let tmp = crate::test_support::unique_test_dir("auto_reverb_rules");
    let _env_guard = crate::test_support::set_local_dir_envs(&tmp);
    let settings = AutoReverbSettings {
        enabled: false,
        rules: BTreeMap::from([
            (
                "snare".to_string(),
                serde_json::json!({"Dragonfly Room Reverb preset": "Small Drum Room"}),
            ),
            ("bass|bs".to_string(), serde_json::Value::Null),
        ]),
    };

    save_auto_reverb_settings(&settings).unwrap();

    assert_eq!(load_auto_reverb_settings(), Some(settings));
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn a_missing_or_malformed_file_loads_as_none() {
    let tmp = crate::test_support::unique_test_dir("bad_auto_reverb_rules");
    let _env_guard = crate::test_support::set_local_dir_envs(&tmp);
    assert_eq!(load_auto_reverb_settings(), None);

    let path = crate::paths::auto_reverb_rules_path().unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, "not json").unwrap();
    assert_eq!(load_auto_reverb_settings(), None);
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn a_file_without_enabled_is_on() {
    let tmp = crate::test_support::unique_test_dir("partial_auto_reverb_rules");
    let _env_guard = crate::test_support::set_local_dir_envs(&tmp);
    let path = crate::paths::auto_reverb_rules_path().unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(path, r#"{"rules": {}}"#).unwrap();

    let loaded = load_auto_reverb_settings().unwrap();
    assert!(loaded.enabled);
    assert!(loaded.rules.is_empty());
    std::fs::remove_dir_all(&tmp).ok();
}
