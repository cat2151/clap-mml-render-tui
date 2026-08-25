use super::*;
use std::path::Path;

mod migration;
mod paths;
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
fn session_state_default_cursor_is_zero() {
    let state = SessionState::default();
    assert_eq!(state.cursor, 0);
}

#[test]
fn session_state_default_lines_is_cde() {
    let state = SessionState::default();
    assert_eq!(state.lines, vec!["cde".to_string()]);
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
fn keyboard_session_defaults_to_x4() {
    let keyboard = KeyboardSessionState::default();
    assert_eq!(keyboard.patch, None);
    assert_eq!(keyboard.buffer_multiplier, 4);
}

#[test]
fn session_state_serialize_deserialize() {
    let state = SessionState {
        cursor: 42,
        lines: vec!["abc".to_string(), "def".to_string()],
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
        mml_overlay_play_settings: MmlOverlayPlaySettings {
            repeat: true,
            modulation: false,
            velocity: true,
        },
    };
    let json = serde_json::to_string_pretty(&state).unwrap();
    let loaded: SessionState = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.cursor, 42);
    assert_eq!(loaded.lines, vec!["abc".to_string(), "def".to_string()]);
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
        cursor: 0,
        lines: vec!["cde".to_string()],
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
        mml_overlay_play_settings: MmlOverlayPlaySettings::default(),
    };
    let json = serde_json::to_string_pretty(&state).unwrap();
    let loaded: SessionState = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.cursor, 0);
    assert_eq!(loaded.lines, vec!["cde".to_string()]);
    assert_eq!(loaded.active_screen, PrimaryScreen::Notepad);
}

#[test]
fn session_state_serialize_deserialize_daw_screen() {
    let state = SessionState {
        cursor: 1,
        lines: vec!["cde".to_string()],
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
        mml_overlay_play_settings: MmlOverlayPlaySettings::default(),
    };
    let json = serde_json::to_string_pretty(&state).unwrap();
    let loaded: SessionState = serde_json::from_str(&json).unwrap();
    assert_eq!(loaded.cursor, 1);
    assert_eq!(loaded.active_screen, PrimaryScreen::Daw);
}

#[test]
fn session_state_json_from_invalid_returns_default() {
    // 不正なJSONはデフォルト値を返す
    let result: SessionState = serde_json::from_str("not json").unwrap_or_default();
    assert_eq!(result.cursor, 0);
    assert_eq!(result.lines, vec!["cde".to_string()]);
    assert_eq!(result.active_screen, PrimaryScreen::Notepad);
}

#[test]
fn session_state_json_missing_field_returns_default() {
    // cursor フィールドがない場合はデフォルト値を返す
    let result: SessionState = serde_json::from_str("{}").unwrap_or_default();
    assert_eq!(result.cursor, 0);
    assert_eq!(result.lines, vec!["cde".to_string()]);
    assert_eq!(result.active_screen, PrimaryScreen::Notepad);
}

#[test]
fn session_state_json_missing_lines_uses_default() {
    // lines フィールドがない場合（旧形式の history.json）はデフォルト値 ["cde"] を返す
    let result: SessionState = serde_json::from_str(r#"{"cursor": 3}"#).unwrap();
    assert_eq!(result.cursor, 3);
    assert_eq!(result.lines, vec!["cde".to_string()]);
}

#[test]
fn session_state_json_missing_screen_defaults_to_notepad() {
    let result: SessionState = serde_json::from_str(r#"{"cursor": 3, "lines": ["cde"]}"#).unwrap();
    assert_eq!(result.cursor, 3);
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

#[test]
fn session_state_json_empty_lines_passes_through_serde() {
    // serde は "lines": [] を空配列のままデシリアライズする（serde デフォルトは適用されない）。
    // load_session_state() がこれを検知して default_lines() で補填する。
    let raw: SessionState = serde_json::from_str(r#"{"cursor": 2, "lines": []}"#).unwrap();
    assert!(raw.lines.is_empty(), "serde は空配列をそのまま通す");
}

#[test]
fn save_and_load_session_state_roundtrip() {
    // 実ユーザーデータディレクトリに影響しないよう、一時ファイルに直接書き込んで
    // JSON シリアライズ/デシリアライズの往復を検証する
    let tmp_path = std::env::temp_dir().join("cmrt_test_history_roundtrip.json");

    let state = SessionState {
        cursor: 7,
        lines: vec!["cde".to_string(), "fga".to_string()],
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
        mml_overlay_play_settings: MmlOverlayPlaySettings::default(),
    };
    let json = serde_json::to_string_pretty(&state).unwrap();
    std::fs::write(&tmp_path, &json).unwrap();

    let read_back = std::fs::read_to_string(&tmp_path).unwrap();
    let loaded: SessionState = serde_json::from_str(&read_back).unwrap();
    std::fs::remove_file(&tmp_path).ok();

    assert_eq!(loaded.cursor, 7);
    assert_eq!(loaded.lines, vec!["cde".to_string(), "fga".to_string()]);
    assert_eq!(loaded.active_screen, PrimaryScreen::Notepad);
}

#[test]
fn save_and_load_session_state_roundtrip_daw_mode() {
    // DAW モードのセッション状態が正しく保存・復元されることを検証する
    let tmp_path = std::env::temp_dir().join("cmrt_test_history_roundtrip_daw.json");

    let state = SessionState {
        cursor: 0,
        lines: vec!["cde".to_string()],
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
        mml_overlay_play_settings: MmlOverlayPlaySettings::default(),
    };
    let json = serde_json::to_string_pretty(&state).unwrap();
    std::fs::write(&tmp_path, &json).unwrap();

    let read_back = std::fs::read_to_string(&tmp_path).unwrap();
    let loaded: SessionState = serde_json::from_str(&read_back).unwrap();
    std::fs::remove_file(&tmp_path).ok();

    assert_eq!(loaded.active_screen, PrimaryScreen::Daw);
}

#[test]
fn save_and_load_session_state_roundtrip_mml_overlay_play_settings() {
    // `Ctrl+L` の 3 値が、保存したファイルを読み直しても同じ組み合わせで戻ること。
    // 3 値のうち一部だけ ON にして、取り違え（別の項目へ入る）も検出する。
    let tmp_path = std::env::temp_dir().join("cmrt_test_history_roundtrip_play_settings.json");

    let state = SessionState {
        mml_overlay_play_settings: MmlOverlayPlaySettings {
            repeat: true,
            modulation: false,
            velocity: true,
        },
        ..SessionState::default()
    };
    let json = serde_json::to_string_pretty(&state).unwrap();
    std::fs::write(&tmp_path, &json).unwrap();

    let read_back = std::fs::read_to_string(&tmp_path).unwrap();
    let loaded: SessionState = serde_json::from_str(&read_back).unwrap();
    std::fs::remove_file(&tmp_path).ok();

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
fn a_history_file_written_before_the_play_settings_existed_loads_with_them_all_off() {
    // 既存ユーザーの history.json にはこのキーが無い。既定は「全部 OFF」＝
    // Stage 7 までと同じ挙動でなければならない。
    let json = r#"{ "cursor": 0, "lines": ["cde"] }"#;
    let loaded: SessionState = serde_json::from_str(json).unwrap();
    assert_eq!(
        loaded.mml_overlay_play_settings,
        MmlOverlayPlaySettings::default()
    );
    assert!(!loaded.mml_overlay_play_settings.repeat);
}
