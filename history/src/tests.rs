use super::*;
use std::path::Path;

mod migration;
mod paths;
mod session_file_roundtrip;
mod storage;
mod voicing_cache;

fn assert_history_file_path(path: &Path, file_name: &str) {
    assert_eq!(
        path.file_name().and_then(|n| n.to_str()),
        Some(file_name),
        "history ファイル名が期待と異なる: {:?}",
        path
    );

    let history_dir = path
        .parent()
        .expect("history ファイルに親ディレクトリがない");
    assert_eq!(
        history_dir.file_name().and_then(|n| n.to_str()),
        Some("history"),
        "history ファイルの親ディレクトリ名が history ではない: {:?}",
        history_dir
    );

    let app_dir = history_dir
        .parent()
        .expect("history ディレクトリにアプリディレクトリがない");
    assert_eq!(
        app_dir.file_name().and_then(|n| n.to_str()),
        Some("clap-mml-render-tui"),
        "history ファイルのアプリディレクトリ名が clap-mml-render-tui ではない: {:?}",
        app_dir
    );
}

#[test]
fn session_state_default_screen_is_notepad() {
    let state = SessionState::default();
    assert_eq!(state.active_screen, PrimaryScreen::Notepad);
}

#[test]
fn session_state_default_has_default_keyboard_state() {
    assert_eq!(
        SessionState::default().keyboard,
        KeyboardSessionState::default()
    );
}

#[test]
fn session_state_default_has_no_keyboard_note_guide_date() {
    assert_eq!(
        SessionState::default().keyboard_note_guide_overlay_date,
        None
    );
}

#[test]
fn session_state_default_has_no_chord_chart_patch() {
    let state = SessionState::default();
    assert_eq!(state.chord_chart_patch, None);
    assert!(state.chord_chart_bass_enabled);
    assert_eq!(state.chord_chart_bass_patch, None);
}

#[test]
fn keyboard_session_defaults_to_x4() {
    let keyboard = KeyboardSessionState::default();
    assert_eq!(keyboard.patch, None);
    assert_eq!(keyboard.buffer_multiplier, 4);
}

#[test]
fn session_state_serialize_deserialize() {
    let state = SessionState {
        active_screen: PrimaryScreen::Notepad,
        keyboard: KeyboardSessionState::default(),
        grid_sequencer_track_count: 16,
        grid_sequencer_chord_mode: false,
        grid_sequencer: None,
        grid_sequencer_bpm: None,
        loop_browser_bpm: None,
        grid_sequencer_bpm_range: None,
        loop_browser_bpm_range: None,
        keyboard_note_guide_overlay_date: Some("2026-07-20".to_string()),
        notepad_sound_check_guide_overlay_date: Some("2026-07-19".to_string()),
        mml_overlay_patch: Some("Leads/Lead 1.fxp".to_string()),
        chord_chart_patch: Some("Keys/Piano.fxp".to_string()),
        chord_chart_bass_enabled: false,
        chord_chart_bass_patch: Some("Bass/Finger Bass.fxp".to_string()),
        chord_chart_query: String::new(),
        chord_chart_bass_query: String::new(),
        chord_chart_effect_chain: Vec::new(),
        mml_overlay_play_settings: MmlOverlayPlaySettings {
            repeat: true,
            modulation: false,
            velocity: true,
        },
    };
    let json = serde_json::to_string_pretty(&state).unwrap();
    let loaded: SessionState = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.active_screen, PrimaryScreen::Notepad);
    assert_eq!(
        loaded.keyboard_note_guide_overlay_date.as_deref(),
        Some("2026-07-20")
    );
    assert_eq!(
        loaded.notepad_sound_check_guide_overlay_date.as_deref(),
        Some("2026-07-19")
    );
    assert_eq!(
        loaded.mml_overlay_patch.as_deref(),
        Some("Leads/Lead 1.fxp")
    );
    assert_eq!(loaded.chord_chart_patch.as_deref(), Some("Keys/Piano.fxp"));
    assert!(!loaded.chord_chart_bass_enabled);
    assert_eq!(
        loaded.chord_chart_bass_patch.as_deref(),
        Some("Bass/Finger Bass.fxp")
    );
    assert_eq!(
        loaded.mml_overlay_play_settings,
        MmlOverlayPlaySettings {
            repeat: true,
            modulation: false,
            velocity: true,
        }
    );
}

#[test]
fn session_state_serialize_deserialize_zero() {
    let state = SessionState {
        active_screen: PrimaryScreen::Notepad,
        keyboard: KeyboardSessionState::default(),
        grid_sequencer_track_count: 16,
        grid_sequencer_chord_mode: false,
        grid_sequencer: None,
        grid_sequencer_bpm: None,
        loop_browser_bpm: None,
        grid_sequencer_bpm_range: None,
        loop_browser_bpm_range: None,
        keyboard_note_guide_overlay_date: None,
        notepad_sound_check_guide_overlay_date: None,
        mml_overlay_patch: None,
        chord_chart_patch: None,
        chord_chart_bass_enabled: true,
        chord_chart_bass_patch: None,
        chord_chart_query: String::new(),
        chord_chart_bass_query: String::new(),
        chord_chart_effect_chain: Vec::new(),
        mml_overlay_play_settings: MmlOverlayPlaySettings::default(),
    };
    let json = serde_json::to_string_pretty(&state).unwrap();
    let loaded: SessionState = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.active_screen, PrimaryScreen::Notepad);
}

#[test]
fn session_state_patch_fields_round_trip_independently() {
    let state = SessionState {
        mml_overlay_patch: Some("Global/Pad.fxp".to_string()),
        chord_chart_patch: Some("Chord Chart/Piano.fxp".to_string()),
        chord_chart_bass_enabled: false,
        chord_chart_bass_patch: Some("Bass/Upright.fxp".to_string()),
        chord_chart_query: "plugin:floe".to_string(),
        chord_chart_bass_query: "bass".to_string(),
        chord_chart_effect_chain: Vec::new(),
        ..SessionState::default()
    };

    let json = serde_json::to_string(&state).unwrap();
    let loaded: SessionState = serde_json::from_str(&json).unwrap();

    assert_eq!(loaded.mml_overlay_patch.as_deref(), Some("Global/Pad.fxp"));
    assert_eq!(
        loaded.chord_chart_patch.as_deref(),
        Some("Chord Chart/Piano.fxp")
    );
    assert!(!loaded.chord_chart_bass_enabled);
    assert_eq!(
        loaded.chord_chart_bass_patch.as_deref(),
        Some("Bass/Upright.fxp")
    );
    assert_eq!(loaded.chord_chart_query, "plugin:floe");
    assert_eq!(loaded.chord_chart_bass_query, "bass");
}

#[test]
fn history_without_chord_chart_patch_keeps_existing_patch_and_defaults_to_none() {
    let json = r#"{
        "cursor": 3,
        "lines": ["cde"],
        "mml_overlay_patch": "Global/Lead.fxp"
    }"#;

    let loaded: SessionState = serde_json::from_str(json).unwrap();

    assert_eq!(loaded.mml_overlay_patch.as_deref(), Some("Global/Lead.fxp"));
    assert_eq!(loaded.chord_chart_patch, None);
    assert!(loaded.chord_chart_bass_enabled);
    assert_eq!(loaded.chord_chart_bass_patch, None);
    assert_eq!(loaded.chord_chart_query, "");
    assert_eq!(loaded.chord_chart_bass_query, "");
}

#[test]
fn session_state_serialize_deserialize_daw_screen() {
    let state = SessionState {
        active_screen: PrimaryScreen::Daw,
        keyboard: KeyboardSessionState::default(),
        grid_sequencer_track_count: 16,
        grid_sequencer_chord_mode: false,
        grid_sequencer: None,
        grid_sequencer_bpm: None,
        loop_browser_bpm: None,
        grid_sequencer_bpm_range: None,
        loop_browser_bpm_range: None,
        keyboard_note_guide_overlay_date: None,
        notepad_sound_check_guide_overlay_date: None,
        mml_overlay_patch: None,
        chord_chart_patch: None,
        chord_chart_bass_enabled: true,
        chord_chart_bass_patch: None,
        chord_chart_query: String::new(),
        chord_chart_bass_query: String::new(),
        chord_chart_effect_chain: Vec::new(),
        mml_overlay_play_settings: MmlOverlayPlaySettings::default(),
    };
    let json = serde_json::to_string_pretty(&state).unwrap();
    let loaded: SessionState = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.active_screen, PrimaryScreen::Daw);
}

#[test]
fn session_state_json_missing_field_returns_default() {
    // 設定が未指定なら既定値を使う
    let result: SessionState = serde_json::from_str("{}").unwrap();
    assert_eq!(result.active_screen, PrimaryScreen::Notepad);
}

#[test]
fn session_state_json_missing_screen_defaults_to_notepad() {
    let result: SessionState = serde_json::from_str(r#"{"cursor": 3, "lines": ["cde"]}"#).unwrap();
    assert_eq!(result.active_screen, PrimaryScreen::Notepad);
}

#[test]
fn session_state_json_missing_keyboard_uses_default() {
    let result: SessionState = serde_json::from_str(r#"{"cursor": 3, "lines": ["cde"]}"#).unwrap();
    assert_eq!(result.keyboard, KeyboardSessionState::default());
    assert_eq!(result.keyboard_note_guide_overlay_date, None);
}

#[test]
fn new_active_screen_takes_precedence_over_legacy_flags() {
    let result: SessionState = serde_json::from_str(
        r#"{
            "cursor": 3,
            "lines": ["cde"],
            "active_screen": "loop_browser",
            "is_daw_mode": true,
            "keyboard": {"patch": "Piano"}
        }"#,
    )
    .unwrap();
    assert_eq!(result.active_screen, PrimaryScreen::LoopBrowser);
    assert_eq!(result.keyboard.patch.as_deref(), Some("Piano"));
}

#[test]
fn legacy_keyboard_takes_precedence_over_legacy_daw_flag() {
    let result: SessionState = serde_json::from_str(
        r#"{
            "cursor": 3,
            "lines": ["cde"],
            "is_daw_mode": true,
            "keyboard": {"patch": "Piano"}
        }"#,
    )
    .unwrap();
    assert_eq!(result.active_screen, PrimaryScreen::Keyboard);
}
