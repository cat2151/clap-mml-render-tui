use super::*;

#[test]
fn clamp_session_cursor_caps_to_last_available_line() {
    assert_eq!(crate::tui::session::clamp_session_cursor(0, 1), 0);
    assert_eq!(crate::tui::session::clamp_session_cursor(2, 3), 2);
    assert_eq!(crate::tui::session::clamp_session_cursor(9, 3), 2);
}

#[test]
fn begin_playback_session_invalidates_previous_session() {
    let app = TuiApp::new_for_test(test_config());

    let first = app.playback_session.begin();
    let second = app.playback_session.begin();

    assert!(!app.playback_session.is_current(first));
    assert!(app.playback_session.is_current(second));
}

#[test]
fn set_play_state_if_current_ignores_stale_session() {
    let app = TuiApp::new_for_test(test_config());

    let stale = app.playback_session.begin();
    let current = app.playback_session.begin();
    let newer = app.playback_session.begin();

    app.playback_session
        .set_play_state_if_current(stale, PlayState::Done("old".to_string()));
    assert!(matches!(
        &*app.playback_session.play_state().lock().unwrap(),
        PlayState::Idle
    ));

    app.playback_session
        .set_play_state_if_current(current, PlayState::Running("new".to_string()));
    assert!(matches!(
        &*app.playback_session.play_state().lock().unwrap(),
        PlayState::Idle
    ));

    app.playback_session
        .set_play_state_if_current(newer, PlayState::Running("new".to_string()));
    assert!(matches!(
        &*app.playback_session.play_state().lock().unwrap(),
        PlayState::Running(msg) if msg == "new"
    ));
}

#[test]
fn save_history_state_persists_tui_cursor_lines_and_active_screen() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_tui_save_history_state_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.notepad.set_session_lines_for_test(vec![
        "abc".to_string(),
        "def".to_string(),
        "ghi".to_string(),
    ]);
    app.notepad.set_session_cursor_for_test(2);
    app.active_screen = crate::screen_switch::PrimaryScreen::Daw;

    app.save_history_state();
    crate::history::save_keyboard_note_guide_overlay_date("2026-07-20").unwrap();
    crate::history::save_notepad_sound_check_guide_overlay_date("2026-07-19").unwrap();

    let history_path = crate::test_utils::session_state_path_for_test()
        .expect("config local dir should resolve in isolated TUI history test");
    assert!(
        history_path.exists(),
        "expected isolated history file to be created at {}",
        history_path.display()
    );
    let saved = crate::history::load_session_state();
    assert_eq!(saved.cursor, 2);
    assert_eq!(saved.lines, app.notepad.session_lines());
    assert_eq!(saved.active_screen, crate::history::PrimaryScreen::Daw);
    assert_eq!(
        saved.keyboard_note_guide_overlay_date.as_deref(),
        Some("2026-07-20")
    );
    assert_eq!(
        saved.notepad_sound_check_guide_overlay_date.as_deref(),
        Some("2026-07-19")
    );

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn keyboard_q_persists_and_restores_patch_and_buffer() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_keyboard_session_restore_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.start_keyboard(Some("patches_factory/Keys/Piano.fxp".to_string()));
    app.handle_keyboard_key_event(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('H'),
        crossterm::event::KeyModifiers::SHIFT,
    ));
    let action = app.handle_keyboard_key_event(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('q'),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(matches!(action, crate::tui::keyboard::KeyboardAction::Quit));
    app.save_history_state();

    let saved = crate::history::load_session_state();
    assert_eq!(saved.active_screen, crate::history::PrimaryScreen::Keyboard);
    assert_eq!(
        saved.keyboard,
        crate::history::KeyboardSessionState {
            patch: Some("patches_factory/Keys/Piano.fxp".to_string()),
            buffer_multiplier: 8,
        }
    );

    let cfg = test_config();
    let mut restored = TuiApp::new(&cfg, cmrt_offline_render::PluginEntries::none());
    assert_eq!(
        restored.active_screen,
        crate::screen_switch::PrimaryScreen::Keyboard
    );
    assert_eq!(
        restored.keyboard.state.patch(),
        Some("patches_factory/Keys/Piano.fxp")
    );
    assert_eq!(restored.keyboard.state.buffer_multiplier(), 8);

    restored.handle_keyboard_key_event(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('n'),
        crossterm::event::KeyModifiers::NONE,
    ));
    restored.save_history_state();
    let saved = crate::history::load_session_state();
    assert_eq!(saved.active_screen, crate::history::PrimaryScreen::Notepad);
    assert_eq!(
        saved.keyboard.patch.as_deref(),
        Some("patches_factory/Keys/Piano.fxp")
    );

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn grid_track_count_is_persisted_and_restored() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_grid_track_count_restore_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::GridSequencer;
    let action = app.handle_grid_sequencer_key_event(crossterm::event::KeyEvent::new(
        crossterm::event::KeyCode::Char('t'),
        crossterm::event::KeyModifiers::NONE,
    ));
    assert!(matches!(
        action,
        crate::tui::grid_sequencer::GridSequencerAction::RestartWithTrackCount(1)
    ));
    app.save_history_state();

    let saved = crate::history::load_session_state();
    assert_eq!(saved.grid_sequencer_track_count, 1);

    let cfg = test_config();
    let restored = TuiApp::new(&cfg, cmrt_offline_render::PluginEntries::none());
    assert_eq!(restored.grid_sequencer.track_count(), 1);

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn edited_grid_is_persisted_and_restored_without_persisting_derived_note() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_edited_grid_restore_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);
    let pattern = crate::tui::grid_sequencer::NotePattern::from_steps((0..16).map(|step| {
        if step % 3 == 0 {
            crate::tui::grid_sequencer::NoteStep::Attack
        } else {
            crate::tui::grid_sequencer::NoteStep::Rest
        }
    }));
    let instance = crate::tui::grid_sequencer::GridInstance {
        patch: Some("Keys/Piano.fxp".to_string()),
        lane_mode: crate::tui::grid_sequencer::GridLaneMode::Single,
        drum: None,
        voicing_rotation: 0,
        swing: 61,
        lanes: vec![crate::tui::grid_sequencer::GridLane {
            base_note: 64,
            pattern: pattern.clone(),
        }],
    };
    // voicing_rotation を持てるのは 4 voice の行（行3 = instance 2）だけ。
    let mut voicing_instance = crate::tui::grid_sequencer::GridInstance::new(2);
    voicing_instance.voicing_rotation = -5;
    let mut app = TuiApp::new_for_test(test_config());
    app.grid_sequencer = crate::tui::grid_sequencer::GridSequencerScreen::new_with(
        crate::tui::grid_sequencer::GridSequencerParts {
            track_count: 4,
            restored_session: Some(crate::tui::grid_sequencer::GridSequencerSession::new(
                vec![
                    instance,
                    crate::tui::grid_sequencer::GridInstance::new(1),
                    voicing_instance,
                    crate::tui::grid_sequencer::GridInstance::new(3),
                ],
                crate::tui::grid_sequencer::CycleRandom::HOLD,
            )),
            ..crate::tui::grid_sequencer::GridSequencerParts::default()
        },
    );
    app.active_screen = crate::screen_switch::PrimaryScreen::GridSequencer;

    app.save_history_state();

    let json = std::fs::read_to_string(
        crate::test_utils::session_state_path_for_test().expect("isolated history path"),
    )
    .unwrap();
    // lane からは `base_note` だけを保存する。発音音高（コードから導出する `note`）を
    // 残すと、コードを変えたときに古い音高が復活する。
    // `cycle_random` にも `note` という項目があるので、lane を見て確かめる。
    let stored: serde_json::Value = serde_json::from_str(&json).unwrap();
    let lanes = stored["grid_sequencer"]["instances"]
        .as_array()
        .expect("instances")
        .iter()
        .flat_map(|instance| instance["lanes"].as_array().expect("lanes"))
        .collect::<Vec<_>>();
    assert!(!lanes.is_empty());
    assert!(
        lanes.iter().all(|lane| lane.get("note").is_none()),
        "derived note must not be stored: {json}"
    );
    assert!(json.contains("\"note_steps\""));
    assert!(!json.contains("\"duration\""));
    assert!(!json.contains("\"cells\""));
    let cfg = test_config();
    let restored = TuiApp::new(&cfg, cmrt_offline_render::PluginEntries::none());
    let session = restored.grid_sequencer.session_state().unwrap();
    assert_eq!(
        session.cycle_random,
        crate::tui::grid_sequencer::CycleRandom::HOLD
    );
    assert_eq!(session.instances.len(), 4);
    assert_eq!(
        session.instances[0].patch.as_deref(),
        Some("Keys/Piano.fxp")
    );
    assert_eq!(session.instances[0].lanes[0].base_note, 64);
    assert_eq!(session.instances[0].lanes[0].pattern, pattern);
    assert_eq!(session.instances[2].voicing_rotation, -5);
    // swing は domain と DTO の変換が手書きなので、往復して初めて繋がったと言える。
    assert_eq!(session.instances[0].swing, 61);

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn daw_mode_switch_request_can_be_consumed_from_tui_runtime() {
    assert!(!crate::daw::take_http_mode_switch_request());

    crate::daw::request_http_mode_switch();

    assert!(crate::daw::take_http_mode_switch_request());
    assert!(!crate::daw::take_http_mode_switch_request());
}

#[test]
fn loop_browser_screen_is_saved_and_restored_as_the_startup_screen() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_loop_browser_session_restore_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.begin_loop_browser_startup();
    app.stop_loop_browser();
    app.save_history_state();

    let saved = crate::history::load_session_state();
    assert_eq!(
        saved.active_screen,
        crate::history::PrimaryScreen::LoopBrowser
    );

    let cfg = test_config();
    let restored = TuiApp::new(&cfg, cmrt_offline_render::PluginEntries::none());
    assert_eq!(
        restored.active_screen,
        crate::history::PrimaryScreen::LoopBrowser
    );
    assert_eq!(
        restored.active_screen,
        crate::screen_switch::PrimaryScreen::LoopBrowser
    );
    assert!(restored.loop_browser.state.starting);

    std::fs::remove_dir_all(&tmp).ok();
}

/// `Mode` から画面種別（Keyboard / LoopBrowser）を外したため、`Mode::Normal` だけでは
/// 「notepad を表示中」を判定できない。notepad 固有の定期処理が他画面でも走らないよう、
/// 画面判定を含む述語を固定しておく。
#[test]
fn notepad_normal_mode_active_is_false_on_other_screens_and_submodes() {
    let mut app = TuiApp::new_for_test(test_config());
    assert!(app.notepad_normal_mode_active());

    app.notepad.mode = Mode::Insert;
    assert!(!app.notepad_normal_mode_active());
    app.notepad.mode = Mode::Normal;

    app.start_keyboard(None);
    assert!(!app.notepad_normal_mode_active());

    app.switch_to_primary_screen(crate::screen_switch::PrimaryScreen::Notepad, None);
    assert!(app.notepad_normal_mode_active());

    app.begin_loop_browser_startup();
    assert!(!app.notepad_normal_mode_active());

    app.active_screen = crate::screen_switch::PrimaryScreen::Daw;
    assert!(!app.notepad_normal_mode_active());
}
