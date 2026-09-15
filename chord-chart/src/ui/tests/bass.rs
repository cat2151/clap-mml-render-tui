use super::*;

#[test]
fn the_screen_title_always_shows_the_bass_preview_state() {
    let mut screen = ChordChartScreen::default();
    assert!(buffer_to_string(&render(&screen)).contains("Bass:ON"));

    screen.set_bass_enabled(false);
    assert!(buffer_to_string(&render(&screen)).contains("Bass:OFF"));
}

#[test]
fn the_bass_state_in_the_title_does_not_take_over_the_error_footer() {
    let mut screen = ChordChartScreen::default();
    screen.set_bass_enabled(false);
    screen.error = Some("プレビュー失敗".to_string());

    let buffer = render(&screen);
    let title = row_text(&buffer, 0);
    let footer = row_text(&buffer, buffer.area.height - 2);

    assert!(title.contains("Bass:OFF"), "{title:?}");
    assert!(squeeze(&footer).contains("プレビュー失敗"), "{footer:?}");
    assert!(!footer.contains("Bass:"), "{footer:?}");
}
