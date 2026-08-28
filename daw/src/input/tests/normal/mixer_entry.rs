use super::*;

#[test]
fn handle_normal_m_enters_mixer_mode_on_playable_track() {
    let (mut app, _cache_rx) = build_test_app();
    app.editor.cursor_track = 0;

    let result = app.handle_normal(crossterm::event::KeyCode::Char('m'));

    assert!(matches!(result, super::super::DawNormalAction::Continue));
    assert!(matches!(app.mode, DawMode::Mixer));
    assert_eq!(app.overlays.mixer.cursor_track, 2);
}
