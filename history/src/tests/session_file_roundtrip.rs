use super::*;

#[test]
fn save_and_load_session_state_roundtrip() {
    // 実ユーザーデータディレクトリに影響しないよう、一時ファイルに直接書き込んで
    // JSON シリアライズ/デシリアライズの往復を検証する
    let tmp_path = crate::test_support::unique_test_dir("history_roundtrip_json");

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
        chord_chart_patch: None,
        chord_chart_bass_enabled: true,
        chord_chart_bass_patch: None,
        chord_chart_query: String::new(),
        chord_chart_bass_query: String::new(),
        chord_chart_effect_chain: Vec::new(),
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
    let tmp_path = crate::test_support::unique_test_dir("history_roundtrip_daw_json");

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
        chord_chart_patch: None,
        chord_chart_bass_enabled: true,
        chord_chart_bass_patch: None,
        chord_chart_query: String::new(),
        chord_chart_bass_query: String::new(),
        chord_chart_effect_chain: Vec::new(),
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
    let tmp_path = crate::test_support::unique_test_dir("history_roundtrip_play_settings_json");

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
    // 設定が無かったころと同じ挙動でなければならない。
    let json = r#"{ "cursor": 0, "lines": ["cde"] }"#;
    let loaded: SessionState = serde_json::from_str(json).unwrap();
    assert_eq!(
        loaded.mml_overlay_play_settings,
        MmlOverlayPlaySettings::default()
    );
    assert!(!loaded.mml_overlay_play_settings.repeat);
}
