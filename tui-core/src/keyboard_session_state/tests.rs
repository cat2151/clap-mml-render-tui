use super::*;

#[test]
fn old_json_without_note_playback_fields_reads_as_off_with_no_target() {
    let state: KeyboardSessionState =
        serde_json::from_str(r#"{"patch":"a.fxp","buffer_multiplier":2}"#).unwrap();

    assert_eq!(state.note_playback_mode, NotePlaybackMode::Off);
    assert!(state.repeat_chords.is_empty());
    assert!(state.mml.is_empty());
    assert_eq!(state.patch.as_deref(), Some("a.fxp"));
    assert_eq!(state.buffer_multiplier, 2);
}

#[test]
fn note_playback_fields_round_trip_through_json() {
    let state = KeyboardSessionState {
        note_playback_mode: NotePlaybackMode::Auto,
        repeat_chords: vec![vec![60, 64, 67], vec![65, 69]],
        mml: "c;e;g f;a".to_string(),
        ..KeyboardSessionState::default()
    };

    let json = serde_json::to_string(&state).unwrap();
    assert!(json.contains(r#""note_playback_mode":"auto""#), "{json}");
    let restored: KeyboardSessionState = serde_json::from_str(&json).unwrap();
    assert_eq!(restored, state);
}

#[test]
fn old_json_without_an_effect_chain_reads_as_empty() {
    let state: KeyboardSessionState =
        serde_json::from_str(r#"{"patch":"a.fxp","buffer_multiplier":2}"#).unwrap();

    assert!(state.effect_chain.is_empty());
}

#[test]
fn a_two_stage_effect_chain_round_trips_through_json() {
    let state = KeyboardSessionState {
        effect_chain: vec![
            serde_json::json!({"fx": "Delay/Echo"}),
            serde_json::json!({"amp": "Clean", "bypass": true}),
        ],
        ..KeyboardSessionState::default()
    };

    let json = serde_json::to_string(&state).unwrap();
    let restored: KeyboardSessionState = serde_json::from_str(&json).unwrap();
    assert_eq!(restored.effect_chain, state.effect_chain);
}
