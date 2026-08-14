use super::*;

#[test]
fn fixed_chord_input_is_drawn_as_the_frontmost_single_line_overlay() {
    let mut screen = GridSequencerScreen::new(None);
    screen.open_chord_input();

    let mut terminal = terminal_for(&screen);
    let rendered = buffer_to_string(&terminal);

    assert!(rendered.contains("Fixed Chord Progression"), "{rendered}");
    assert!(rendered.contains("key:G Isus4-I"), "{rendered}");
    assert!(rendered.contains("Enter:"), "{rendered}");
    assert!(terminal
        .backend()
        .buffer()
        .area
        .contains(terminal.get_cursor_position().unwrap()));
}
