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
    terminal
        .draw(|f| draw(&mut screen, &ready, std::time::Instant::now(), f))
        .unwrap();
    terminal
}

fn row_text(terminal: &Terminal<TestBackend>, y: u16) -> String {
    let buffer = terminal.backend().buffer();
    (0..buffer.area.width)
        .map(|x| buffer.cell((x, y)).unwrap().symbol())
        .collect()
}

/// 2 行目の `t:` より後で ` {label}` が最初に現れる、語の先頭セルの前景色。
fn label_color(terminal: &Terminal<TestBackend>, label: &str) -> Color {
    let row = row_text(terminal, 1);
    let header = row.find("t:").expect("note mode line") + "t:".len();
    let start = header
        + row[header..]
            .find(&format!(" {label}"))
            .unwrap_or_else(|| panic!("{label} not in {row}"));
    let x = row[..start + 1].chars().count() as u16;
    terminal.backend().buffer().cell((x, 1)).unwrap().fg
}

#[test]
fn the_keyboard_pane_is_top_left_and_its_first_row_is_the_note_mode_line() {
    let terminal = render_screen(KeyboardState::default());

    assert!(
        row_text(&terminal, 0).starts_with("┌ [KEYBOARD] ─"),
        "{}",
        row_text(&terminal, 0)
    );
    assert!(
        row_text(&terminal, 1).starts_with("│t: off  auto  repeat  arp "),
        "{}",
        row_text(&terminal, 1)
    );
    assert!(row_text(&terminal, 2).starts_with("│ "));
    assert!(row_text(&terminal, 3).starts_with("│PC key:"));
    assert_eq!(label_color(&terminal, "off"), MONOKAI_GREEN);
    for label in ["auto", "repeat", "arp"] {
        assert_eq!(label_color(&terminal, label), MONOKAI_GRAY, "{label}");
    }
    let screen = buffer_to_string(&terminal);
    for gone in [
        "Note mode",
        "keyboard mode",
        "1-9:count",
        "Active:",
        "Patch:",
        "Target:",
    ] {
        assert!(!screen.contains(gone), "{gone}: {screen}");
    }
    assert!(screen.contains("Chord: -"), "{screen}");
}

#[test]
fn the_note_mode_line_colors_the_current_mode_and_names_what_auto_does() {
    let mut state = KeyboardState::default();
    state.set_detected_voicing(cmrt_realtime_play::PatchVoicing::Mono);
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    let _ = state.cycle_note_playback(std::time::Instant::now());

    let terminal = render_screen(state);
    assert!(
        row_text(&terminal, 1).starts_with("│t: off  auto→arp  repeat  arp"),
        "{}",
        row_text(&terminal, 1)
    );
    assert_eq!(label_color(&terminal, "auto→arp"), MONOKAI_GREEN);
    assert_eq!(label_color(&terminal, "off"), MONOKAI_GRAY);
    assert_eq!(label_color(&terminal, "arp "), MONOKAI_GRAY);
}

/// 枠の内側に全行が入らない高さでは空行を削り、`t` の行からコントローラ行・和音の行までを残す。
#[test]
fn a_short_keyboard_pane_drops_blank_rows_before_content_rows() {
    let mut screen = crate::KeyboardScreen::new(
        None,
        KeyboardState::default(),
        crate::KeyboardMmlInput::default(),
        crate::KeyboardNoteGuide::new(None),
    );
    // pane の高さ 10（内側 8 行）= 全体 14 - status 1 - help 3。
    let ready = crate::KeyboardConnectionStatus {
        phase: crate::KeyboardConnectionPhase::Ready,
        ..Default::default()
    };
    let mut terminal = Terminal::new(TestBackend::new(160, 14)).unwrap();
    terminal
        .draw(|f| draw(&mut screen, &ready, std::time::Instant::now(), f))
        .unwrap();

    let expected = [
        (1, "│t: off"),
        (2, "│PC key:"),
        (3, "│Note:"),
        (4, "│vel: 100"),
        (5, "│mod: off"),
        (6, "│pb: +8191"),
        (7, "│cc#(x): 1"),
        (9, "└"),
    ];
    for (y, prefix) in expected {
        let row = row_text(&terminal, y);
        assert!(row.starts_with(prefix), "y={y}: {row}");
    }
}

/// 行 `y` の `x` から始まる各セルの (文字, 前景色)。
fn cells_from(terminal: &Terminal<TestBackend>, y: u16, x: u16, len: u16) -> Vec<(String, Color)> {
    (x..x + len)
        .map(|x| {
            let cell = terminal.backend().buffer().cell((x, y)).unwrap();
            (cell.symbol().to_string(), cell.fg)
        })
        .collect()
}

#[test]
fn only_the_shortcut_key_letters_are_colored_on_the_controller_and_note_mode_rows() {
    let terminal = render_screen(KeyboardState::default());
    // (行, 中身, ショートカット色のセルの位置)
    let rows: [(u16, &str, &[usize]); 4] = [
        (6, "vel: 100  127  cyc", &[0]),
        (7, "mod: off  on  cyc", &[0]),
        (8, "pb: +8191  0  -8192  0  cyc  0", &[0]),
        (9, "cc#(x): 1  Z: off  cyc", &[4, 11]),
    ];
    for (y, text, keys) in rows {
        assert!(
            row_text(&terminal, y).starts_with(&format!("│{text} ")),
            "{}",
            row_text(&terminal, y)
        );
        for (offset, (symbol, fg)) in cells_from(&terminal, y, 1, text.len() as u16)
            .into_iter()
            .enumerate()
        {
            if keys.contains(&offset) {
                assert_eq!(fg, MONOKAI_CYAN, "y={y} {symbol} at {offset}");
            } else {
                assert_ne!(fg, MONOKAI_CYAN, "y={y} {symbol} at {offset}");
            }
        }
    }
    let note_mode = cells_from(&terminal, 1, 1, 2);
    assert_eq!(note_mode[0], ("t".to_string(), MONOKAI_CYAN));
    assert_ne!(note_mode[1].1, MONOKAI_CYAN);
}

#[test]
fn the_controller_rows_color_only_the_current_choice() {
    let terminal = render_screen(KeyboardState::default());
    // 既定は vel=100・mod=off・cc=off が選ばれ、pb はまだ何も選ばれていない。
    for (y, x, expected) in [
        (6, 6, MONOKAI_GREEN),
        (6, 11, MONOKAI_GRAY),
        (7, 6, MONOKAI_GREEN),
        (7, 11, MONOKAI_GRAY),
        (8, 5, MONOKAI_GRAY),
        (9, 15, MONOKAI_GREEN),
        (9, 20, MONOKAI_GRAY),
    ] {
        let cell = terminal.backend().buffer().cell((x, y)).unwrap();
        assert_eq!(cell.fg, expected, "({x},{y}) {}", cell.symbol());
    }
}
