use super::*;

#[test]
fn load_daw_session_state_reads_history_daw_json() {
    let tmp = std::env::temp_dir().join("cmrt_test_history_daw_load");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let state = DawSessionState {
        cursor_track: 3,
        cursor_measure: 4,
        cached_measures: vec![DawCachedMeasure {
            track: 2,
            measure: 5,
            mml_hash: daw_cache_mml_hash("t120cdef"),
            legacy_mml: None,
        }],
    };
    save_daw_session_state(&state).unwrap();

    assert_eq!(load_daw_session_state(), state);
    let saved_path = super::daw_session_state_path().unwrap();
    assert_eq!(
        saved_path.parent(),
        Some(super::history_dir().unwrap().as_path())
    );
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn load_daw_session_state_migrates_from_history_json() {
    let tmp = std::env::temp_dir().join("cmrt_test_history_daw_migrate");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let history_dir = super::history_dir().unwrap();
    std::fs::create_dir_all(&history_dir).unwrap();
    let history_path = history_dir.join("history.json");
    std::fs::write(
        &history_path,
        r#"{
  "cursor": 7,
  "lines": ["cde"],
  "is_daw_mode": true,
  "daw_cursor_track": 4,
  "daw_cursor_measure": 2,
  "daw_cached_measures": [
    { "track": 1, "measure": 3, "mml": "t120gab" }
  ]
}"#,
    )
    .unwrap();

    let daw_state = load_daw_session_state();
    assert_eq!(
        daw_state,
        DawSessionState {
            cursor_track: 4,
            cursor_measure: 2,
            cached_measures: vec![DawCachedMeasure {
                track: 1,
                measure: 3,
                mml_hash: daw_cache_mml_hash("t120gab"),
                legacy_mml: None,
            }],
        }
    );

    let migrated_history = std::fs::read_to_string(&history_path).unwrap();
    assert!(!migrated_history.contains("daw_cursor_track"));
    assert!(!migrated_history.contains("daw_cached_measures"));

    let history_daw_path = super::daw_session_state_path().unwrap();
    let stored = std::fs::read_to_string(&history_daw_path).unwrap();
    assert!(stored.contains("\"cursor_track\": 4"));
    assert!(stored.contains("\"cursor_measure\": 2"));
    assert!(stored.contains("\"mml_hash\""));
    assert!(!stored.contains("t120gab"));

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn patch_phrase_store_serialize_deserialize_roundtrip() {
    let mut store = PatchPhraseStore {
        notepad: PatchPhraseState {
            history: vec!["cde".to_string()],
            favorites: vec!["gab".to_string()],
        },
        favorite_patches: vec!["Pads/Soft Pad.fxp".to_string()],
        ..Default::default()
    };
    store.patches.insert(
        "Pads/Soft Pad.fxp".to_string(),
        PatchPhraseState {
            history: vec!["o4c".to_string(), "o5g".to_string()],
            favorites: vec!["l8cdef".to_string()],
        },
    );

    let json = serde_json::to_string_pretty(&store).unwrap();
    let loaded: PatchPhraseStore = serde_json::from_str(&json).unwrap();

    assert_eq!(loaded, store);
}

#[test]
fn save_and_load_patch_phrase_store_roundtrip() {
    let tmp = std::env::temp_dir().join("cmrt_test_patch_phrase_store_roundtrip");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut store = PatchPhraseStore {
        notepad: PatchPhraseState {
            history: vec!["abc".to_string()],
            favorites: vec!["xyz".to_string()],
        },
        favorite_patches: vec!["Leads/Lead 1.fxp".to_string()],
        ..Default::default()
    };
    store.patches.insert(
        "Leads/Lead 1.fxp".to_string(),
        PatchPhraseState {
            history: vec!["c".to_string()],
            favorites: vec!["g".to_string(), "o5c".to_string()],
        },
    );

    save_patch_phrase_store(&store).unwrap();

    assert_eq!(load_patch_phrase_store(), store);
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn load_session_state_migrates_from_legacy_data_local_history_json() {
    let tmp = std::env::temp_dir().join("cmrt_test_legacy_history_json_migrate");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let legacy_path = crate::test_utils::legacy_session_state_path_for_test().unwrap();
    std::fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
    std::fs::write(
        &legacy_path,
        r#"{
  "cursor": 9,
  "lines": ["abc", "def"],
  "is_daw_mode": true
}"#,
    )
    .unwrap();

    let state = load_session_state();
    assert_eq!(state.cursor, 9);
    assert_eq!(state.lines, vec!["abc".to_string(), "def".to_string()]);
    assert!(state.is_daw_mode);

    let new_path = super::session_state_path().unwrap();
    assert!(
        new_path.exists(),
        "migrated history.json が新配置に存在しない"
    );
    let migrated: SessionState =
        serde_json::from_str(&std::fs::read_to_string(&new_path).unwrap()).unwrap();
    assert_eq!(migrated.cursor, 9);
    assert_eq!(migrated.lines, vec!["abc".to_string(), "def".to_string()]);
    assert!(migrated.is_daw_mode);

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn load_daw_session_state_migrates_from_legacy_history_daw_json() {
    let tmp = std::env::temp_dir().join("cmrt_test_legacy_history_daw_json_migrate");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let legacy_path = crate::test_utils::legacy_daw_session_state_path_for_test().unwrap();
    std::fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
    std::fs::write(
        &legacy_path,
        r#"{
  "cursor_track": 2,
  "cursor_measure": 6,
  "cached_measures": [
    { "track": 1, "measure": 4, "mml_hash": 12345 }
  ]
}"#,
    )
    .unwrap();

    let state = load_daw_session_state();
    assert_eq!(state.cursor_track, 2);
    assert_eq!(state.cursor_measure, 6);
    assert_eq!(state.cached_measures.len(), 1);
    assert_eq!(state.cached_measures[0].mml_hash, 12345);

    let new_path = super::daw_session_state_path().unwrap();
    assert!(
        new_path.exists(),
        "migrated history_daw.json が新配置に存在しない"
    );

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn load_patch_phrase_store_migrates_from_legacy_patch_history_json() {
    let tmp = std::env::temp_dir().join("cmrt_test_legacy_patch_history_json_migrate");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let legacy_path = crate::test_utils::legacy_patch_phrase_store_path_for_test().unwrap();
    std::fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
    std::fs::write(
        &legacy_path,
        r#"{
  "notepad": {
    "history": ["abc"],
    "favorites": ["xyz"]
  },
  "patches": {
    "Pads/Soft Pad.fxp": {
      "history": ["l8cdef"],
      "favorites": ["o5g"]
    }
  }
}"#,
    )
    .unwrap();

    let store = load_patch_phrase_store();
    assert_eq!(store.notepad.history, vec!["abc".to_string()]);
    assert_eq!(store.notepad.favorites, vec!["xyz".to_string()]);
    assert_eq!(
        store.patches.get("Pads/Soft Pad.fxp").unwrap().history,
        vec!["l8cdef".to_string()]
    );
    assert!(store.favorite_patches.is_empty());

    let new_path = super::patch_phrase_store_path().unwrap();
    assert!(
        new_path.exists(),
        "migrated patch_history.json が新配置に存在しない"
    );

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn daw_file_load_path_migrates_from_legacy_daw_json() {
    let tmp = std::env::temp_dir().join("cmrt_test_legacy_daw_json_migrate");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let legacy_path = crate::test_utils::legacy_daw_file_path_for_test().unwrap();
    std::fs::create_dir_all(legacy_path.parent().unwrap()).unwrap();
    std::fs::write(&legacy_path, r#"{"tracks":[]}"#).unwrap();

    let migrated_path = super::daw_file_load_path().unwrap();
    assert_history_file_path(&migrated_path, "daw.json");
    assert!(
        migrated_path.exists(),
        "migrated daw.json が新配置に存在しない"
    );
    assert_eq!(
        std::fs::read_to_string(&migrated_path).unwrap(),
        r#"{"tracks":[]}"#
    );

    std::fs::remove_dir_all(&tmp).ok();
}
