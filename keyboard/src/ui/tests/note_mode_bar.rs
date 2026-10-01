use super::*;
use cmrt_tui_core::theme::MONOKAI_GRAY;

fn render_screen(state: KeyboardState) -> Terminal<TestBackend> {
    let mut screen = crate::KeyboardScreen::new(
        None,
        state,
        crate::KeyboardMmlInput::default(),
        crate::KeyboardNoteGuide::new(None),
    );
    let ready = crate::KeyboardConnectionStatus {
        phase: crate::KeyboardConnectionPhase::Ready,
        ..Default::default()
    };
    let mut terminal = Terminal::new(TestBackend::new(160, 24)).unwrap();
    terminal.draw(|f| draw(&mut screen, &ready, f)).unwrap();
    terminal
}

fn row_text(terminal: &Terminal<TestBackend>, y: u16) -> String {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.width)
        .map(|x| buffer.cell((x, y)).unwrap().symbol())
        .collect()
}

/// 2 行目の `t: repeat:` より後で ` {label}` が最初に現れる、語の先頭セルの前景色。
fn label_color(terminal: &Terminal<TestBackend>, label: &str) -> Color {
    let row = row_text(terminal, 1);
    let header = row.find("t: repeat:").expect("note mode bar") + "t: repeat:".len();
    let start = header
        + row[header..]
            .find(&format!(" {label}"))
            .unwrap_or_else(|| panic!("{label} not in {row}"));
    let x = row[..start + 1].chars().count() as u16;
    terminal.backend().buffer().cell((x, 1)).unwrap().fg
}

#[test]
fn the_note_mode_bar_sits_in_a_framed_row_at_the_top_left() {
    let terminal = render_screen(KeyboardState::default());

    assert!(
        row_text(&terminal, 0).starts_with('┌'),
        "{}",
        row_text(&terminal, 0)
    );
    assert!(
        row_text(&terminal, 1).starts_with("│t: repeat: off  auto  repeat  arp"),
        "{}",
        row_text(&terminal, 1)
    );
    assert!(
        row_text(&terminal, 2).starts_with('└'),
        "{}",
        row_text(&terminal, 2)
    );
    assert!(row_text(&terminal, 3).contains("[KEYBOARD]"));
    assert_eq!(label_color(&terminal, "off"), MONOKAI_GREEN);
    for label in ["auto", "repeat", "arp"] {
        assert_eq!(label_color(&terminal, label), MONOKAI_GRAY, "{label}");
    }
    let screen = buffer_to_string(&terminal);
    assert!(!screen.contains("Note mode"), "{screen}");
    assert!(screen.contains("Target: -"), "{screen}");
}

#[test]
fn the_note_mode_bar_colors_the_current_mode_and_names_what_auto_does() {
    let mut state = KeyboardState::default();
    state.set_detected_voicing(cmrt_realtime_play::PatchVoicing::Mono);
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    let _ = state.cycle_note_playback(std::time::Instant::now());

    let terminal = render_screen(state);
    assert!(
        row_text(&terminal, 1).starts_with("│t: repeat: off  auto→arp  repeat  arp"),
        "{}",
        row_text(&terminal, 1)
    );
    assert_eq!(label_color(&terminal, "auto→arp"), MONOKAI_GREEN);
    assert_eq!(label_color(&terminal, "off"), MONOKAI_GRAY);
    assert_eq!(label_color(&terminal, "arp "), MONOKAI_GRAY);
}
