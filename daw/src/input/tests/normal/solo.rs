use super::*;

#[test]
fn handle_normal_s_enables_solo_for_current_track() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;
    app.editor.data[0][0] = r#"{"beat": "4/4"}t120"#.to_string();
    app.editor.data[2][0] = r#"{"Surge XT patch": "piano"}"#.to_string();
    app.editor.data[2][1] = "cde".to_string();
    app.editor.data[3][0] = r#"{"Surge XT patch": "brass"}"#.to_string();
    app.editor.data[3][1] = "gab".to_string();

    app.handle_normal(crossterm::event::KeyCode::Char('s'));

    assert_eq!(app.solo_tracks, vec![false, false, true, false]);
    assert!(app.solo_mode_active());
    assert!(app.playback.measure_mmls.lock().unwrap()[0].contains("cde"));
    assert!(!app.playback.measure_mmls.lock().unwrap()[0].contains("gab"));
}

#[test]
fn handle_normal_s_toggles_tracks_and_turns_off_solo_mode_when_all_false() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 2;

    app.handle_normal(crossterm::event::KeyCode::Char('s'));
    assert_eq!(app.solo_tracks, vec![false, false, true, false]);

    app.editor.cursor_track = 3;
    app.handle_normal(crossterm::event::KeyCode::Char('s'));
    assert_eq!(app.solo_tracks, vec![false, false, true, true]);

    app.editor.cursor_track = 2;
    app.handle_normal(crossterm::event::KeyCode::Char('s'));
    assert_eq!(app.solo_tracks, vec![false, false, false, true]);
    assert!(app.solo_mode_active());

    app.editor.cursor_track = 3;
    app.handle_normal(crossterm::event::KeyCode::Char('s'));
    assert_eq!(app.solo_tracks, vec![false, false, false, false]);
    assert!(!app.solo_mode_active());
}
