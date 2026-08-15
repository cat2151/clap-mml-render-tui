use super::*;

#[test]
fn screen_bpm_modes_are_persisted_and_restored_independently() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_screen_bpm_restore_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.grid_sequencer = crate::tui::grid_sequencer::GridSequencerScreen::new_with(
        crate::tui::grid_sequencer::GridSequencerParts {
            bpm_mode: cmrt_tui_core::bpm::BpmMode::Manual(127.125),
            ..crate::tui::grid_sequencer::GridSequencerParts::default()
        },
    );
    app.loop_browser
        .state
        .set_bpm_mode(cmrt_tui_core::bpm::BpmMode::Manual(93.75));

    app.save_history_state();

    let saved = crate::history::load_session_state();
    assert_eq!(saved.grid_sequencer_bpm, Some(127.125));
    assert_eq!(saved.loop_browser_bpm, Some(93.75));

    let cfg = test_config();
    let restored = TuiApp::new(&cfg, None);
    assert_eq!(
        restored.grid_sequencer.bpm_mode(),
        cmrt_tui_core::bpm::BpmMode::Manual(127.125)
    );
    assert_eq!(
        restored.loop_browser.state.bpm_mode(),
        cmrt_tui_core::bpm::BpmMode::Manual(93.75)
    );

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn screen_bpm_ranges_are_persisted_and_restored_independently() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_screen_bpm_range_restore_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let grid_range = cmrt_tui_core::bpm::BpmRange::new(80.0, 160.0).unwrap();
    let loop_range = cmrt_tui_core::bpm::BpmRange::new(90.0, 140.0).unwrap();
    let mut app = TuiApp::new_for_test(test_config());
    app.grid_sequencer = crate::tui::grid_sequencer::GridSequencerScreen::new_with(
        crate::tui::grid_sequencer::GridSequencerParts {
            bpm_range: grid_range,
            ..crate::tui::grid_sequencer::GridSequencerParts::default()
        },
    );
    app.loop_browser.state.set_bpm_range(loop_range);

    app.save_history_state();

    let saved = crate::history::load_session_state();
    assert_eq!(saved.grid_sequencer_bpm_range, Some([80.0, 160.0]));
    assert_eq!(saved.loop_browser_bpm_range, Some([90.0, 140.0]));

    let cfg = test_config();
    let restored = TuiApp::new(&cfg, None);
    assert_eq!(restored.grid_sequencer.bpm_range(), grid_range);
    assert_eq!(restored.loop_browser.state.bpm_range(), loop_range);
    // 引いた BPM は保存しない。起動時に範囲から引き直す。
    assert!(
        (80.0..=160.0).contains(&restored.grid_sequencer.bpm()),
        "{}",
        restored.grid_sequencer.bpm()
    );
    let loop_bpm = restored.loop_browser.state.bpm_mode().bpm();
    assert!((90.0..=140.0).contains(&loop_bpm), "{loop_bpm}");

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn the_default_bpm_ranges_are_not_written_to_the_session() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_default_bpm_range_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let app = TuiApp::new_for_test(test_config());
    app.save_history_state();

    let saved = crate::history::load_session_state();
    assert_eq!(saved.grid_sequencer_bpm_range, None);
    assert_eq!(saved.loop_browser_bpm_range, None);

    std::fs::remove_dir_all(&tmp).ok();
}
