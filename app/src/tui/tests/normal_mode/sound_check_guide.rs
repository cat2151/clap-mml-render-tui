use super::*;

fn show_notepad_sound_check_overlay(app: &mut TuiApp<'_>) {
    let now = std::time::Instant::now();
    app.notepad_sound_check_guide.tick(now, true, "2026-07-20");
    app.notepad_sound_check_guide
        .tick(now + std::time::Duration::from_secs(1), true, "2026-07-20");
}

#[test]
fn plain_j_and_k_complete_notepad_sound_check_guide() {
    for key in ['j', 'k'] {
        let mut app = TuiApp::new_for_test(test_config());
        show_notepad_sound_check_overlay(&mut app);

        app.handle_normal_key_event(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE));

        assert_eq!(
            app.notepad_sound_check_guide.presentation(),
            crate::sound_check_guide::SoundCheckGuidePresentation::Hidden
        );
    }
}

#[test]
fn modified_j_and_arrow_key_do_not_complete_notepad_sound_check_guide() {
    let mut app = TuiApp::new_for_test(test_config());
    show_notepad_sound_check_overlay(&mut app);

    app.handle_normal_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::SHIFT));
    app.handle_normal_key_event(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));

    assert_eq!(
        app.notepad_sound_check_guide.presentation(),
        crate::sound_check_guide::SoundCheckGuidePresentation::Overlay
    );
}
