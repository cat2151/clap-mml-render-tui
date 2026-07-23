use super::*;

#[test]
fn daw_draws_the_shared_screen_switch_overlay() {
    let mut app = build_test_app();
    app.overlays.screen_switch.open();

    let screen = render_lines(&app, 80, 20).join("\n");

    assert!(screen.contains("Screen Switch"));
    assert!(screen.contains("[N] Notepad"));
    assert!(screen.contains("[D] DAW"));
    assert!(screen.contains("[K] Keyboard"));
    assert!(screen.contains("[L] Loop Browser"));
}
