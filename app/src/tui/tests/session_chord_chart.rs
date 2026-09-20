use super::*;

fn bass_toggle() -> KeyEvent {
    KeyEvent::new(KeyCode::Char('B'), KeyModifiers::SHIFT)
}

fn isolated_history(name: &str) -> (std::path::PathBuf, crate::test_utils::TestEnvGuard) {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_{name}_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let guards = crate::test_utils::set_local_dir_envs(&tmp);
    (tmp, guards)
}

#[test]
fn bass_preview_settings_survive_a_restart_independently() {
    let (tmp, _guards) = isolated_history("chord_chart_bass_session");
    let cfg = test_config();
    let mut app = TuiApp::new_for_test(cfg.clone());
    app.chord_chart.set_bass_enabled(false);
    app.chord_chart_patch = Some("Chord/Piano.fxp".to_string());
    app.chord_chart_bass_patch = Some("Bass/Finger Bass.fxp".to_string());
    app.save_history_state();
    drop(app);

    let saved = crate::history::load_session_state();
    assert!(!saved.chord_chart_bass_enabled);
    assert_eq!(
        saved.chord_chart_bass_patch.as_deref(),
        Some("Bass/Finger Bass.fxp")
    );

    let restored = TuiApp::new(&cfg, cmrt_offline_render::EffectPlugins::none());
    assert!(!restored.chord_chart.bass_enabled());
    assert_eq!(
        restored.chord_chart_bass_patch.as_deref(),
        Some("Bass/Finger Bass.fxp")
    );
    assert_eq!(
        restored.chord_chart_patch.as_deref(),
        Some("Chord/Piano.fxp")
    );

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn bass_toggle_persists_immediately_as_a_preview_setting() {
    let (tmp, _guards) = isolated_history("chord_chart_bass_toggle");
    let mut app = TuiApp::new_for_test(test_config());
    assert!(app.chord_chart.bass_enabled());

    assert_eq!(
        app.handle_chord_chart_key_event(bass_toggle()),
        crate::tui::chord_chart::ChordChartAction::PreviewSettingChanged
    );

    assert!(!crate::history::load_session_state().chord_chart_bass_enabled);
    std::fs::remove_dir_all(&tmp).ok();
}
