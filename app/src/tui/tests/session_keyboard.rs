use super::*;

fn press(app: &mut TuiApp<'_>, code: KeyCode) {
    app.handle_keyboard_key_event(KeyEvent::new(code, KeyModifiers::NONE));
}

fn repeat_chord_notes(app: &TuiApp<'_>) -> Vec<Vec<u8>> {
    app.keyboard
        .state
        .repeat_chords()
        .iter()
        .map(|chord| chord.iter().map(|note| note.midi_note).collect())
        .collect()
}

#[test]
fn keyboard_note_mode_target_and_mml_survive_quit_and_restart() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_keyboard_note_mode_restore_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.start_keyboard(Some("patches_factory/Keys/Piano.fxp".to_string()));
    press(&mut app, KeyCode::Char('i'));
    for ch in "C-F".chars() {
        press(&mut app, KeyCode::Char(ch));
    }
    press(&mut app, KeyCode::Enter);
    let progression = repeat_chord_notes(&app);
    assert!(progression.len() >= 2, "{progression:?}");
    let now = std::time::Instant::now();
    let _ = app.keyboard.state.cycle_note_playback(now);
    let _ = app.keyboard.state.cycle_note_playback(now);
    assert_eq!(
        app.keyboard.state.note_playback_mode(),
        crate::tui::keyboard::NotePlaybackMode::Repeat
    );
    press(&mut app, KeyCode::Char('q'));
    app.save_history_state();

    let saved = crate::history::load_session_state().unwrap();
    assert_eq!(
        saved.keyboard.note_playback_mode,
        crate::tui::keyboard::NotePlaybackMode::Repeat
    );
    assert_eq!(saved.keyboard.repeat_chords, progression);
    assert_eq!(saved.keyboard.mml, "C-F");

    let cfg = test_config();
    let mut restored = TuiApp::new(&cfg, cmrt_offline_render::EffectPlugins::none()).unwrap();
    assert_eq!(
        restored.keyboard.state.note_playback_mode(),
        crate::tui::keyboard::NotePlaybackMode::Repeat
    );
    assert_eq!(repeat_chord_notes(&restored), progression);
    press(&mut restored, KeyCode::Char('i'));
    assert_eq!(restored.keyboard.mml_input.value(), "C-F");
    press(&mut restored, KeyCode::Esc);

    // notepad から入り直しても t のモードと対象は残る
    restored.start_keyboard(Some("patches_factory/Keys/Piano.fxp".to_string()));
    assert_eq!(
        restored.keyboard.state.note_playback_mode(),
        crate::tui::keyboard::NotePlaybackMode::Repeat
    );
    assert_eq!(repeat_chord_notes(&restored), progression);

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn the_keyboard_screen_gets_the_app_effect_catalog() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_keyboard_effect_catalog_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let cfg = test_config();
    let app = TuiApp::new(&cfg, super::chord_chart_auto_reverb::effect_plugins()).unwrap();
    let catalog = app.keyboard.effect_plugins().catalog();
    assert!(catalog.is_some_and(|catalog| !catalog.presets().is_empty()));

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn the_keyboard_effect_chain_survives_quit_and_restart() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_keyboard_effect_chain_restore_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let cfg = test_config();
    let mut app = TuiApp::new(&cfg, super::chord_chart_auto_reverb::effect_plugins()).unwrap();
    app.start_keyboard(Some("patches_factory/Keys/Piano.fxp".to_string()));
    press(&mut app, KeyCode::Char('l'));
    press(&mut app, KeyCode::Char('a'));
    press(&mut app, KeyCode::Char('j'));
    press(&mut app, KeyCode::Enter);
    let chain = app.keyboard.effect_pane().chain().to_vec();
    assert_eq!(chain.len(), 1, "{chain:?}");
    press(&mut app, KeyCode::Char('q'));
    app.save_history_state();

    assert_eq!(
        crate::history::load_session_state()
            .unwrap()
            .keyboard
            .effect_chain,
        chain
    );
    let restored = TuiApp::new(&cfg, super::chord_chart_auto_reverb::effect_plugins()).unwrap();
    assert_eq!(restored.keyboard.effect_pane().chain(), chain);
    // 最初の音色の準備に同梱する chain。
    assert_eq!(restored.keyboard.effect_pane().sounding(), chain);

    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn keyboard_controller_selections_survive_quit_and_restart() {
    use crate::tui::keyboard::{ModulationMode, PitchBendMode, VelocityMode};

    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_keyboard_controller_restore_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.start_keyboard(Some("patches_factory/Keys/Piano.fxp".to_string()));
    press(&mut app, KeyCode::Char('v'));
    press(&mut app, KeyCode::Char('v'));
    for code in [KeyCode::Char('x'), KeyCode::Char('7'), KeyCode::Char('4')] {
        press(&mut app, code);
    }
    press(&mut app, KeyCode::Enter);
    // m / p / Z のキーは接続が Ready のときだけ効く。テストには server が無いので、
    // キーが呼ぶのと同じ遷移を直接進める。
    let now = std::time::Instant::now();
    let _ = app.keyboard.state.cycle_modulation(now);
    let _ = app.keyboard.state.cycle_pitch_bend(now);
    let _ = app.keyboard.state.cycle_pitch_bend(now);
    let _ = app.keyboard.state.toggle_cc_periodic(now);
    press(&mut app, KeyCode::Char('q'));
    app.save_history_state();

    let saved = crate::history::load_session_state()
        .unwrap()
        .keyboard
        .controllers;
    assert_eq!(saved.velocity, VelocityMode::Periodic);
    assert_eq!(saved.modulation, ModulationMode::On);
    assert_eq!(saved.pitch_bend, PitchBendMode::CenterAfterMax);
    assert_eq!(saved.cc_number, 74);
    assert!(saved.cc_periodic);

    let cfg = test_config();
    let mut restored = TuiApp::new(&cfg, cmrt_offline_render::EffectPlugins::none()).unwrap();
    let state = &mut restored.keyboard.state;
    assert_eq!(state.velocity_mode(), VelocityMode::Periodic);
    assert_eq!(state.modulation_mode(), ModulationMode::On);
    assert_eq!(state.pitch_bend_mode(), PitchBendMode::CenterAfterMax);
    assert_eq!(state.cc_number(), 74);
    assert!(state.cc_periodic_on());
    // 最初の Ready で現在値を送り直す
    let refresh = state.take_pending_refresh_messages(now);
    assert!(refresh.contains(&[0xB0, 1, 127]), "{refresh:?}");
    assert!(refresh.contains(&[0xE0, 0x00, 0x40]), "{refresh:?}");
    assert!(refresh.contains(&[0xB0, 74, 127]), "{refresh:?}");

    std::fs::remove_dir_all(&tmp).ok();
}
