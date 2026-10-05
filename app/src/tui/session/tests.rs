use super::*;

#[test]
fn daily_daw_is_saved_and_restored_as_the_cold_start_screen() {
    let unique = crate::tui::tests::NEXT_TEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_daily_daw_session_restore_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(crate::tui::tests::test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::DailyDaw;
    app.save_history_state();
    drop(app);

    let saved = crate::history::load_session_state();
    assert_eq!(saved.active_screen, crate::history::PrimaryScreen::DailyDaw);
    let cfg = crate::tui::tests::test_config();
    let restored = TuiApp::new(&cfg, cmrt_offline_render::EffectPlugins::none());
    assert_eq!(
        restored.active_screen,
        crate::screen_switch::PrimaryScreen::DailyDaw
    );
    assert_eq!(
        crate::tui::runtime::DawEntryRoute::Restored(restored.active_screen).screen(),
        Some(crate::screen_switch::PrimaryScreen::DailyDaw)
    );

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn drum_sequencer_restores_only_active_screen_with_empty_silent_matrix() {
    let unique = crate::tui::tests::NEXT_TEST_ID.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_drum_session_{}_{}",
        std::process::id(),
        unique
    ));
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);
    let mut app = TuiApp::new_for_test(crate::tui::tests::test_config());
    app.switch_to_primary_screen(crate::screen_switch::PrimaryScreen::DrumSequencer, None);
    app.drum_sequencer
        .screen
        .set_kit("Kit.sfz".to_string(), Some(vec![36]));
    app.dispatch_drum_sequencer_key_event(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Enter,
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(app.drum_sequencer.screen.cell_on(36, 0));
    app.save_history_state();
    drop(app);
    let saved = crate::history::load_session_state();
    assert_eq!(
        saved.active_screen,
        crate::history::PrimaryScreen::DrumSequencer
    );
    let encoded = serde_json::to_value(&saved).unwrap();
    assert_eq!(encoded["active_screen"], "drum_sequencer");
    assert!(
        encoded.get("drum_sequencer").is_none(),
        "phrase has no session field"
    );
    let cfg = crate::tui::tests::test_config();
    let restored = TuiApp::new(&cfg, cmrt_offline_render::EffectPlugins::none());
    assert_eq!(
        restored.active_screen,
        crate::screen_switch::PrimaryScreen::DrumSequencer
    );
    assert!(restored.drum_sequencer.screen.kit_name().is_none());
    assert!(!restored.drum_sequencer.screen.cell_on(36, 0));
    assert!(!restored.drum_sequencer.selector_open());
    assert!(restored.drum_sequencer.preview_command.is_none());
    assert_eq!(
        restored
            .mml_overlay_sender
            .as_ref()
            .unwrap()
            .status()
            .command_id(),
        0
    );
    drop(restored);
    std::fs::remove_dir_all(&tmp).ok();
}
