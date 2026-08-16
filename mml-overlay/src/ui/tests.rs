use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{backend::TestBackend, Terminal};

use super::*;

fn render(overlay: &MmlOverlay<'_>) -> String {
    let mut terminal = Terminal::new(TestBackend::new(40, 12)).unwrap();
    terminal.draw(|frame| draw(overlay, frame)).unwrap();
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            (0..buffer.area.width)
                .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn overlay_with(input: &str) -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.open(Vec::new());
    let now = Instant::now();
    for code in input.chars().map(KeyCode::Char) {
        overlay.handle_key(KeyEvent::new(code, KeyModifiers::NONE), now);
    }
    overlay
}

#[test]
fn draws_the_title_and_the_typed_mml() {
    let rendered = render(&overlay_with("cde"));

    assert!(rendered.contains("MML"), "{rendered}");
    assert!(rendered.contains("Esc:close"), "{rendered}");
    assert!(rendered.contains("cde"), "{rendered}");
}

#[test]
fn shows_the_sounding_note_name() {
    let rendered = render(&overlay_with("c"));

    assert!(rendered.contains("sounding: c5"), "{rendered}");
}

#[test]
fn shows_every_member_of_a_sounding_chord() {
    let rendered = render(&overlay_with("'ceg'"));

    assert!(rendered.contains("sounding: c5 e5 g5"), "{rendered}");
}

#[test]
fn shows_a_placeholder_before_anything_is_typed() {
    let mut overlay = MmlOverlay::default();
    overlay.open(Vec::new());
    let rendered = render(&overlay);

    assert!(rendered.contains("cde"), "{rendered}");
    assert!(rendered.contains("sounding: -"), "{rendered}");
}

#[test]
fn shows_the_selected_patch_in_the_title() {
    let mut overlay = MmlOverlay::default();
    overlay.set_restored_patch(Some("Leads/Lead 1.fxp".to_string()));
    overlay.open(Vec::new());
    let rendered = render(&overlay);

    assert!(rendered.contains("Lead 1.fxp"), "{rendered}");
}

#[test]
fn draws_the_patch_select_over_the_input() {
    let mut overlay = MmlOverlay::default();
    overlay.open(vec![(
        "Leads/Lead 1.fxp".to_string(),
        "leads/lead 1.fxp".to_string(),
    )]);
    overlay.handle_key(
        KeyEvent::new(KeyCode::Char('t'), KeyModifiers::CONTROL),
        Instant::now(),
    );
    let rendered = render(&overlay);

    // 全角文字はセル単位で分かれて見えるので、ASCII の部分だけで確かめる。
    assert!(rendered.contains("Enter:"), "{rendered}");
    assert!(rendered.contains("Lead 1.fxp"), "{rendered}");
}

#[test]
fn note_name_uses_the_mml_octave_numbering() {
    assert_eq!(note_name(60), "c5");
    assert_eq!(note_name(61), "c+5");
    assert_eq!(note_name(72), "c6");
}
