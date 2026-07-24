use super::*;

#[test]
fn load_daw_session_state_reads_history_daw_json() {
    let tmp = std::env::temp_dir().join("cmrt_test_history_daw_load");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_support::set_local_dir_envs(&tmp);

    let state = DawSessionState {
        cursor_track: 3,
        cursor_measure: 4,
        cached_measures: vec![DawCachedMeasure {
            track: 2,
            measure: 5,
            mml_hash: daw_cache_mml_hash("t120cdef"),
            legacy_mml: None,
        }],
        daw_sound_check_guide_overlay_date: Some("2026-07-20".to_string()),
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
fn save_daw_sound_check_date_preserves_other_daw_session_fields() {
    let tmp = std::env::temp_dir().join("cmrt_test_history_daw_guide_date");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_support::set_local_dir_envs(&tmp);
    let state = DawSessionState {
        cursor_track: 3,
        cursor_measure: 4,
        cached_measures: vec![DawCachedMeasure {
            track: 2,
            measure: 5,
            mml_hash: 123,
            legacy_mml: None,
        }],
        daw_sound_check_guide_overlay_date: None,
    };
    save_daw_session_state(&state).unwrap();

    save_daw_sound_check_guide_overlay_date("2026-07-20").unwrap();

    let saved = load_daw_session_state();
    assert_eq!(saved.cursor_track, 3);
    assert_eq!(saved.cursor_measure, 4);
    assert_eq!(saved.cached_measures, state.cached_measures);
    assert_eq!(
        saved.daw_sound_check_guide_overlay_date.as_deref(),
        Some("2026-07-20")
    );
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
    let _env_guards = crate::test_support::set_local_dir_envs(&tmp);

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
fn load_session_state_normalizes_keyboard_restore_values() {
    let tmp = std::env::temp_dir().join("cmrt_test_keyboard_history_normalize");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_support::set_local_dir_envs(&tmp);

    let path = super::session_state_path().unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        r#"{
  "cursor": 1,
  "lines": ["cde"],
  "is_daw_mode": true,
  "keyboard": {
    "patch": "  ",
    "transport": "http",
    "buffer_multiplier": 3
  }
}"#,
    )
    .unwrap();

    let state = load_session_state();
    assert_eq!(state.active_screen, PrimaryScreen::Keyboard);
    assert_eq!(
        state.keyboard,
        KeyboardSessionState {
            patch: None,
            transport: KeyboardTransport::Http,
            buffer_multiplier: 4,
        }
    );

    std::fs::remove_dir_all(&tmp).ok();
}
