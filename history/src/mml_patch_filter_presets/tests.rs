use super::*;

#[test]
fn presets_round_trip_as_structured_json() {
    let tmp = std::env::temp_dir().join("cmrt_test_mml_patch_filter_presets");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guard = crate::test_support::set_local_dir_envs(&tmp);
    let presets = vec![
        ("lead".to_string(), r"\bviolin".to_string()),
        ("chord".to_string(), r"\bharp".to_string()),
    ];

    save_mml_patch_filter_presets(&presets).unwrap();

    assert_eq!(load_mml_patch_filter_presets(), presets);
    let json = std::fs::read_to_string(
        crate::paths::mml_patch_filter_presets_path().expect("test path is available"),
    )
    .unwrap();
    assert!(json.contains(r#""group": "lead""#), "{json}");
    assert!(json.contains(r#""pattern": "\\bviolin""#), "{json}");
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn load_ignores_a_malformed_file() {
    let tmp = std::env::temp_dir().join("cmrt_test_bad_mml_patch_filter_presets");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guard = crate::test_support::set_local_dir_envs(&tmp);
    let path = crate::paths::mml_patch_filter_presets_path().unwrap();
    std::fs::write(path, "not json").unwrap();

    assert!(load_mml_patch_filter_presets().is_empty());
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn save_trims_and_deduplicates_entries() {
    let tmp = std::env::temp_dir().join("cmrt_test_normalized_mml_patch_filter_presets");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guard = crate::test_support::set_local_dir_envs(&tmp);

    save_mml_patch_filter_presets(&[
        (" lead ".to_string(), " violin ".to_string()),
        ("lead".to_string(), "violin".to_string()),
        ("lead".to_string(), " ".to_string()),
    ])
    .unwrap();

    assert_eq!(
        load_mml_patch_filter_presets(),
        vec![("lead".to_string(), "violin".to_string())]
    );
    std::fs::remove_dir_all(&tmp).ok();
}
