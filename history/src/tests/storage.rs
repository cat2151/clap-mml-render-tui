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
            buffer_multiplier: 4,
        }
    );
    assert_eq!(state.grid_sequencer_track_count, 16);
    assert!(
        !state.grid_sequencer_chord_mode,
        "chord mode を知らない古い history は off 扱い"
    );
    assert!(
        state.grid_sequencer.is_none(),
        "grid rows を知らない古い history は未保存扱い"
    );

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn session_state_round_trips_the_grid_sequencer_chord_mode() {
    let tmp = std::env::temp_dir().join("cmrt_test_grid_chord_mode_round_trip");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_support::set_local_dir_envs(&tmp);

    save_session_state(&SessionState {
        grid_sequencer_chord_mode: true,
        ..SessionState::default()
    })
    .unwrap();

    assert!(load_session_state().grid_sequencer_chord_mode);

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn session_state_round_trips_independent_manual_bpms() {
    let tmp = std::env::temp_dir().join("cmrt_test_bpm_round_trip");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_support::set_local_dir_envs(&tmp);

    save_session_state(&SessionState {
        grid_sequencer_bpm: Some(128.123456789),
        loop_browser_bpm: Some(91.75),
        ..SessionState::default()
    })
    .unwrap();

    let loaded = load_session_state();
    assert_eq!(loaded.grid_sequencer_bpm, Some(128.123456789));
    assert_eq!(loaded.loop_browser_bpm, Some(91.75));

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn invalid_saved_bpms_fall_back_to_auto() {
    let state: SessionState =
        serde_json::from_str(r#"{"grid_sequencer_bpm":19.9,"loop_browser_bpm":301}"#).unwrap();

    assert_eq!(state.grid_sequencer_bpm, None);
    assert_eq!(state.loop_browser_bpm, None);
}

#[test]
fn session_state_round_trips_independent_automatic_bpm_ranges() {
    let tmp = std::env::temp_dir().join("cmrt_test_bpm_range_round_trip");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_support::set_local_dir_envs(&tmp);

    save_session_state(&SessionState {
        grid_sequencer_bpm_range: Some([80.0, 160.0]),
        loop_browser_bpm_range: Some([90.0, 140.0]),
        ..SessionState::default()
    })
    .unwrap();

    let loaded = load_session_state();
    assert_eq!(loaded.grid_sequencer_bpm_range, Some([80.0, 160.0]));
    assert_eq!(loaded.loop_browser_bpm_range, Some([90.0, 140.0]));

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn invalid_saved_bpm_ranges_fall_back_to_the_fixed_default() {
    // 下限が範囲外 / 上下が逆 / 小数の端は、どれも範囲として成立しない。
    for json in [
        r#"{"grid_sequencer_bpm_range":[19,160],"loop_browser_bpm_range":[80,301]}"#,
        r#"{"grid_sequencer_bpm_range":[160,80],"loop_browser_bpm_range":[80.5,160]}"#,
    ] {
        let state: SessionState = serde_json::from_str(json).unwrap();
        assert_eq!(state.grid_sequencer_bpm_range, None, "json={json}");
        assert_eq!(state.loop_browser_bpm_range, None, "json={json}");
    }
}

#[test]
fn session_state_round_trips_the_editable_grid() {
    let tmp = std::env::temp_dir().join("cmrt_test_editable_grid_round_trip");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_support::set_local_dir_envs(&tmp);
    let grid = GridSequencerSessionState {
        fixed_chord: None,
        instances: vec![GridSequencerInstanceState {
            patch: Some("Keys/Piano.fxp".to_string()),
            lane_mode: GridLaneModeState::Single,
            // track 4 の drum 行は役割が抽選なので、保存して初めて起動をまたげる。
            drum: Some(crate::GridDrumRoleState::HiHat),
            voicing_rotation: 0,
            swing: 63,
            lanes: vec![GridSequencerLaneState {
                base_note: 67,
                note_steps: (0..16)
                    .map(|step| {
                        if step % 4 == 0 {
                            GridNoteStepState::Attack
                        } else {
                            GridNoteStepState::Tie
                        }
                    })
                    .collect(),
            }],
        }],
        cycle_random: GridCycleRandomState::default(),
    };

    save_session_state(&SessionState {
        grid_sequencer: Some(grid.clone()),
        ..SessionState::default()
    })
    .unwrap();

    assert_eq!(load_session_state().grid_sequencer, Some(grid));

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn load_session_state_normalizes_grid_sequencer_track_count() {
    let tmp = std::env::temp_dir().join("cmrt_test_grid_track_count_normalize");
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_support::set_local_dir_envs(&tmp);

    let path = super::session_state_path().unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(
        &path,
        r#"{
  "lines": ["cde"],
  "active_screen": "grid_sequencer",
  "grid_sequencer_track_count": 5
}"#,
    )
    .unwrap();

    // 3 は chord mode 用に足したので通る。5 のような未対応値だけが既定へ落ちる。
    assert_eq!(load_session_state().grid_sequencer_track_count, 16);

    std::fs::remove_dir_all(&tmp).ok();
}
