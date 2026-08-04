use super::*;

fn render_with_size(screen: &GridSequencerScreen, width: u16, height: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    let connection = screen.connection_status();
    terminal.draw(|f| draw(screen, &connection, f)).unwrap();
    buffer_to_string(&terminal)
}

#[test]
fn short_terminals_keep_the_full_note_grid_and_show_both_value_grids_below_it() {
    let screen = GridSequencerScreen::new(None);

    let rendered = render(&screen);

    assert!(rendered.contains(" 16 "), "{rendered}");
    assert!(rendered.contains("CC1 Modulation"), "{rendered}");
    assert!(rendered.contains("Velocity"), "{rendered}");
    assert!(rendered.contains("SHM idle"), "{rendered}");
}

/// パターン名は実発音中の小節のものなので、鳴り始めるまでタイトルには出ない。
#[test]
fn the_value_grid_titles_carry_the_pattern_of_the_sounding_measure() {
    let now = Instant::now();
    let mut screen = GridSequencerScreen::with_track_count(None, 1);
    screen.state.rows_mut()[0].cells[0] = true;

    let idle = render(&screen);
    assert!(idle.contains(" CC1 Modulation "), "{idle}");
    assert!(!idle.contains("CC1 Modulation ["), "{idle}");

    screen.state.start(now);
    screen.state.poll_steps(now, Duration::ZERO);
    let sounding = render(&screen);

    assert!(
        ["[random]", "[cc1 up]", "[cc1 down]"]
            .iter()
            .any(|label| sounding.contains(label)),
        "{sounding}"
    );
    assert!(
        ["[random]", "[vel up]", "[vel down]"]
            .iter()
            .any(|label| sounding.contains(label)),
        "{sounding}"
    );
}

#[test]
fn tall_terminals_show_every_track_of_both_value_grids() {
    let screen = GridSequencerScreen::new(None);

    let rendered = render_with_size(&screen, 90, 60);
    let cc1 = rendered.split("CC1 Modulation").nth(1).unwrap();
    let (cc1, velocity) = cc1.split_once("Velocity").unwrap();

    assert!(cc1.contains(" 16 "), "{rendered}");
    assert!(velocity.contains(" 16 "), "{rendered}");
    assert!(rendered.contains("SHM idle"), "{rendered}");
}
