use super::*;

#[test]
fn question_mark_opens_help_for_the_focused_pane() {
    let mut browser = browser();

    assert!(matches!(
        browser.handle_key(KeyCode::Char('?')),
        LoopBrowserAction::Continue
    ));
    assert_eq!(browser.help_overlay, Some(LoopBrowserPane::Tree));

    browser.handle_key(KeyCode::Char('?'));
    browser.handle_key(KeyCode::Tab);
    browser.handle_key(KeyCode::Char('?'));

    assert_eq!(browser.focus, LoopBrowserPane::Tracks);
    assert_eq!(browser.help_overlay, Some(LoopBrowserPane::Tracks));
}

#[test]
fn escape_q_and_question_mark_close_help_without_leaving_loop_browser() {
    for close_key in [KeyCode::Esc, KeyCode::Char('q'), KeyCode::Char('?')] {
        let mut browser = browser();
        browser.handle_key(KeyCode::Char('?'));

        assert!(matches!(
            browser.handle_key(close_key),
            LoopBrowserAction::Continue
        ));
        assert_eq!(browser.help_overlay, None);
        assert_eq!(browser.focus, LoopBrowserPane::Tree);
    }
}

#[test]
fn help_blocks_keys_from_reaching_the_underlying_pane() {
    let mut browser = browser();
    let cursor = browser.cursor;
    browser.handle_key(KeyCode::Char('?'));

    browser.handle_key(KeyCode::Char('j'));
    browser.handle_key(KeyCode::Tab);

    assert_eq!(browser.cursor, cursor);
    assert_eq!(browser.focus, LoopBrowserPane::Tree);
    assert_eq!(browser.help_overlay, Some(LoopBrowserPane::Tree));
}

#[test]
fn question_mark_does_not_replace_category_or_mixer_overlays() {
    let mut browser = browser();
    browser.category_overlay = Some(LoopDirId::new(
        std::path::Path::new("/loops"),
        std::path::Path::new("Pack"),
    ));

    browser.handle_key(KeyCode::Char('?'));

    assert!(browser.category_overlay.is_some());
    assert_eq!(browser.help_overlay, None);

    browser.category_overlay = None;
    browser.focus = LoopBrowserPane::Tracks;
    browser.handle_key(KeyCode::Char('m'));
    browser.handle_key(KeyCode::Char('?'));

    assert!(browser.mixer_overlay_open);
    assert_eq!(browser.help_overlay, None);
}
