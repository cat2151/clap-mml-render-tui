use super::*;

#[test]
fn screen_switch_overlay_lists_every_primary_screen() {
    let mut app = TuiApp::new_for_test(test_config());
    app.screen_switch_menu.open();

    let screen = render_lines(&mut app, 80, 20).join("\n");

    assert!(screen.contains("Screen Switch"));
    assert!(screen.contains("[N] Notepad"));
    assert!(screen.contains("[A] Daily DAW"));
    assert!(screen.contains("[D] DAW"));
    assert!(screen.contains("[K] Keyboard"));
    assert!(screen.contains("[L] Loop Browser"));
    assert!(screen.contains("[G] Grid Sequencer"));
    assert!(screen.contains("Esc:cancel"));
}
