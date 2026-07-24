use std::time::{Duration, Instant};

use ratatui::style::Modifier;

use super::*;
use crate::NOTEPAD_SOUND_CHECK_GUIDE_MESSAGE;
use cmrt_tui_core::sound_check_guide::{SoundCheckGuide, SoundCheckGuidePresentation};

fn set_notepad_guide(app: &mut NotepadScreen<'_>, saved_date: Option<&str>) {
    app.sound_check_guide = SoundCheckGuide::new(saved_date.map(str::to_owned));
    let now = Instant::now();
    app.sound_check_guide.tick(now, true, "2026-07-20");
    app.sound_check_guide
        .tick(now + Duration::from_secs(1), true, "2026-07-20");
}

#[test]
fn notepad_daily_sound_check_overlay_is_centered_and_colored() {
    let mut app = NotepadScreen::new_for_test(test_config());
    set_notepad_guide(&mut app, None);

    let buffer = render_buffer(&mut app, 100, 24);
    let screen = render_lines(&mut app, 100, 24).join("\n").replace(' ', "");
    let (x, y) = find_text_ignoring_spaces(&buffer, NOTEPAD_SOUND_CHECK_GUIDE_MESSAGE);
    let cell = buffer.cell((x, y)).unwrap();

    assert!(screen.contains("音出し確認"));
    assert!(screen.contains(NOTEPAD_SOUND_CHECK_GUIDE_MESSAGE));
    assert_eq!(cell.fg, MONOKAI_YELLOW);
    assert!(cell.modifier.contains(Modifier::BOLD));
}

#[test]
fn notepad_same_day_sound_check_replaces_footer() {
    let mut app = NotepadScreen::new_for_test(test_config());
    set_notepad_guide(&mut app, Some("2026-07-20"));

    assert_eq!(
        app.sound_check_guide.presentation(),
        SoundCheckGuidePresentation::Footer
    );
    let screen = render_lines(&mut app, 100, 24).join("\n").replace(' ', "");
    assert!(screen.contains(NOTEPAD_SOUND_CHECK_GUIDE_MESSAGE));
    assert!(!screen.contains("q?:helpe:config"));
    assert!(!screen.contains("音出し確認"));
}

#[test]
fn notepad_sound_check_overlay_is_hidden_outside_normal_mode() {
    let mut app = NotepadScreen::new_for_test(test_config());
    set_notepad_guide(&mut app, None);
    app.mode = Mode::Insert;

    let screen = render_lines(&mut app, 100, 24).join("\n").replace(' ', "");
    assert!(!screen.contains(NOTEPAD_SOUND_CHECK_GUIDE_MESSAGE));
    assert!(!screen.contains("音出し確認"));
}
