use super::*;

fn show_daw_sound_check_overlay(app: &mut DawApp) {
    let now = std::time::Instant::now();
    app.sound_check_guide.tick(now, true, "2026-07-20");
    app.sound_check_guide
        .tick(now + std::time::Duration::from_secs(1), true, "2026-07-20");
}

#[test]
fn plain_hjkl_complete_daw_sound_check_guide() {
    for key in ['h', 'j', 'k', 'l'] {
        let (mut app, _cache_rx) = build_test_app();
        show_daw_sound_check_overlay(&mut app);

        app.handle_normal_key_event(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE));

        assert_eq!(
            app.sound_check_guide.presentation(),
            crate::sound_check_guide::SoundCheckGuidePresentation::Hidden
        );
    }
}

#[test]
fn modified_h_and_arrow_key_do_not_complete_daw_sound_check_guide() {
    let (mut app, _cache_rx) = build_test_app();
    show_daw_sound_check_overlay(&mut app);

    app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::SHIFT));
    app.handle_normal_key_event(KeyEvent::new(KeyCode::Right, KeyModifiers::NONE));

    assert_eq!(
        app.sound_check_guide.presentation(),
        crate::sound_check_guide::SoundCheckGuidePresentation::Overlay
    );
}
