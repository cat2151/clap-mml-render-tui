use super::*;

#[test]
fn handle_normal_question_mark_enters_help_mode() {
    let (mut app, _cache_rx) = build_test_app();

    let result = app.handle_normal(crossterm::event::KeyCode::Char('?'));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(app.mode, DawMode::Help));
    assert!(matches!(app.help_origin, DawMode::Normal));
}

#[test]
fn handle_normal_n_returns_to_tui() {
    let (mut app, _cache_rx) = build_test_app();

    let result = app.handle_normal(crossterm::event::KeyCode::Char('n'));

    assert!(matches!(result, super::super::DawNormalAction::ReturnToTui));
    assert!(matches!(app.mode, DawMode::Normal));
}

#[test]
fn handle_normal_esc_has_no_effect() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 2;
    app.cursor_measure = 1;

    let result = app.handle_normal(crossterm::event::KeyCode::Esc);

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(app.mode, DawMode::Normal));
    assert_eq!(app.cursor_track, 2);
    assert_eq!(app.cursor_measure, 1);
}

#[test]
fn handle_normal_a_cycles_ab_repeat_and_tracks_cursor_until_end_is_fixed() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_measure = 1;

    app.handle_normal(crossterm::event::KeyCode::Char('a'));
    assert_eq!(
        app.ab_repeat_state(),
        AbRepeatState::FixStart {
            start_measure_index: 0,
            end_measure_index: 0,
        }
    );

    app.handle_normal(crossterm::event::KeyCode::Right);
    assert_eq!(
        app.ab_repeat_state(),
        AbRepeatState::FixStart {
            start_measure_index: 0,
            end_measure_index: 1,
        }
    );

    app.handle_normal(crossterm::event::KeyCode::Char('a'));
    assert_eq!(
        app.ab_repeat_state(),
        AbRepeatState::FixEnd {
            start_measure_index: 0,
            end_measure_index: 1,
        }
    );

    app.handle_normal(crossterm::event::KeyCode::Left);
    assert_eq!(
        app.ab_repeat_state(),
        AbRepeatState::FixEnd {
            start_measure_index: 0,
            end_measure_index: 1,
        }
    );

    app.handle_normal(crossterm::event::KeyCode::Char('a'));
    assert_eq!(app.ab_repeat_state(), AbRepeatState::Off);
}

#[test]
fn handle_normal_a_can_turn_off_ab_repeat_from_init_column() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_measure = 1;
    app.handle_normal(crossterm::event::KeyCode::Char('a'));
    app.handle_normal(crossterm::event::KeyCode::Char('a'));

    app.cursor_measure = 0;
    app.handle_normal(crossterm::event::KeyCode::Char('a'));

    assert_eq!(app.ab_repeat_state(), AbRepeatState::Off);
}

#[test]
fn handle_normal_s_enables_solo_for_current_track() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.data[0][0] = r#"{"beat": "4/4"}t120"#.to_string();
    app.data[1][0] = r#"{"Surge XT patch": "piano"}"#.to_string();
    app.data[1][1] = "cde".to_string();
    app.data[2][0] = r#"{"Surge XT patch": "brass"}"#.to_string();
    app.data[2][1] = "gab".to_string();

    app.handle_normal(crossterm::event::KeyCode::Char('s'));

    assert_eq!(app.solo_tracks, vec![false, true, false]);
    assert!(app.solo_mode_active());
    assert!(app.play_measure_mmls.lock().unwrap()[0].contains("cde"));
    assert!(!app.play_measure_mmls.lock().unwrap()[0].contains("gab"));
}

#[test]
fn handle_normal_s_toggles_tracks_and_turns_off_solo_mode_when_all_false() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;

    app.handle_normal(crossterm::event::KeyCode::Char('s'));
    assert_eq!(app.solo_tracks, vec![false, true, false]);

    app.cursor_track = 2;
    app.handle_normal(crossterm::event::KeyCode::Char('s'));
    assert_eq!(app.solo_tracks, vec![false, true, true]);

    app.cursor_track = 1;
    app.handle_normal(crossterm::event::KeyCode::Char('s'));
    assert_eq!(app.solo_tracks, vec![false, false, true]);
    assert!(app.solo_mode_active());

    app.cursor_track = 2;
    app.handle_normal(crossterm::event::KeyCode::Char('s'));
    assert_eq!(app.solo_tracks, vec![false, false, false]);
    assert!(!app.solo_mode_active());
}

#[test]
fn handle_normal_m_enters_mixer_mode_on_playable_track() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 0;

    let result = app.handle_normal(crossterm::event::KeyCode::Char('m'));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(app.mode, DawMode::Mixer));
    assert_eq!(app.mixer_cursor_track, 1);
}

#[test]
fn handle_normal_h_and_j_preview_new_target_when_not_playing() {
    let (mut app, _cache_rx) = build_test_app();
    app.cursor_track = 1;
    app.cursor_measure = 2;
    app.data[1][1] = "cdef".to_string();
    app.data[2][1] = "gabc".to_string();

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.cursor_measure, 1);
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.play_position
            .lock()
            .unwrap()
            .as_ref()
            .map(|position| position.measure_index),
        Some(0)
    );

    let result = app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert_eq!(app.cursor_track, 2);
    assert!(matches!(
        *app.play_state.lock().unwrap(),
        DawPlayState::Preview
    ));
    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("preview: meas1")
    );
}
