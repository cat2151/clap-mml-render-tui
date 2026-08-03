use super::*;

fn render_with_size(screen: &GridSequencerScreen, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    let connection = screen.connection_status();
    terminal.draw(|f| draw(screen, &connection, f)).unwrap();
    buffer_to_string(&terminal)
}

#[test]
fn short_terminals_keep_the_full_note_grid_and_show_the_cc1_group_below_it() {
    let screen = GridSequencerScreen::new(None);

    let rendered = render(&screen);

    assert!(rendered.contains(" 16 "), "{rendered}");
    assert!(rendered.contains("CC1 Modulation"), "{rendered}");
    assert!(rendered.contains("SHM idle"), "{rendered}");
}

#[test]
fn tall_terminals_show_every_cc1_track() {
    let screen = GridSequencerScreen::new(None);

    let rendered = render_with_size(&screen, 90, 40);
    let cc1 = rendered.split("CC1 Modulation").nth(1).unwrap();

    assert!(cc1.contains(" 16 "), "{rendered}");
    assert!(rendered.contains("SHM idle"), "{rendered}");
}
