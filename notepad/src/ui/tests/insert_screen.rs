use super::*;

#[test]
fn insert_screen_shows_insert_title_without_duplicate_line_text() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.editor.lines = vec!["abc".to_string()];
    app.start_insert();

    let lines = render_lines(&mut app, 80, 8);
    let screen = lines.join("\n");

    assert!(screen.contains("[INSERT] notepad mode"));
    assert_eq!(screen.matches("abc").count(), 1);
    assert!(lines.iter().any(|line| line.contains("▶ abc")));
}
