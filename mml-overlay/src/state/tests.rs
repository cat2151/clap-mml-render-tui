mod patch;

use super::*;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn opened() -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.open(Vec::new());
    overlay
}

#[test]
fn typing_a_note_sends_note_on() {
    let mut overlay = opened();
    let now = Instant::now();

    assert_eq!(
        overlay.handle_key(press(KeyCode::Char('c')), now),
        MmlOverlayAction::Send(vec![[0x90, 60, 127]])
    );
    assert_eq!(overlay.sounding(), [60]);
}

#[test]
fn typing_the_next_note_stops_the_previous_one_first() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('c')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Char('d')), now),
        MmlOverlayAction::Send(vec![[0x80, 60, 0], [0x90, 62, 127]])
    );
}

#[test]
fn moving_the_cursor_left_sounds_the_earlier_note_again() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('c')), now);
    overlay.handle_key(press(KeyCode::Char('d')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Left), now),
        MmlOverlayAction::Send(vec![[0x80, 62, 0], [0x90, 60, 127]])
    );
}

#[test]
fn a_modifier_resounds_the_same_note_shifted() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('c')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Char('+')), now),
        MmlOverlayAction::Send(vec![[0x80, 60, 0], [0x90, 61, 127]])
    );
}

#[test]
fn a_command_that_adds_no_note_stays_silent() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('c')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Char('8')), now),
        MmlOverlayAction::Continue
    );
    assert_eq!(overlay.sounding(), [60]);
}

#[test]
fn a_chord_sounds_every_member_at_once() {
    let mut overlay = opened();
    let now = Instant::now();
    for code in "'ce".chars().map(KeyCode::Char) {
        overlay.handle_key(press(code), now);
    }

    assert_eq!(
        overlay.handle_key(press(KeyCode::Char('g')), now),
        MmlOverlayAction::Send(vec![
            [0x80, 60, 0],
            [0x80, 64, 0],
            [0x90, 60, 127],
            [0x90, 64, 127],
            [0x90, 67, 127],
        ])
    );
}

#[test]
fn the_gate_stops_the_note_after_it_expires() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('c')), now);

    assert_eq!(overlay.poll(now + GATE - Duration::from_millis(1)), None);
    assert_eq!(overlay.poll(now + GATE), Some(vec![[0x80, 60, 0]]));
    assert!(overlay.sounding().is_empty());
    assert_eq!(overlay.poll(now + GATE), None);
}

#[test]
fn closing_stops_every_sounding_note() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('c')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Esc), now),
        MmlOverlayAction::Close(vec![[0x80, 60, 0]])
    );
    assert!(!overlay.is_open());
    assert!(overlay.sounding().is_empty());
}

#[test]
fn enter_does_not_insert_a_newline() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('c')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Enter), now),
        MmlOverlayAction::Continue
    );
    assert_eq!(
        overlay.handle_key(
            KeyEvent::new(KeyCode::Char('m'), KeyModifiers::CONTROL),
            now
        ),
        MmlOverlayAction::Continue
    );
    assert_eq!(overlay.value(), "c");
}

#[test]
fn reopening_starts_from_an_empty_input() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('c')), now);
    overlay.handle_key(press(KeyCode::Esc), now);

    overlay.open(Vec::new());
    assert_eq!(overlay.value(), "");
    assert!(overlay.is_open());
}

#[test]
fn deleting_the_last_note_and_typing_it_again_sounds_it() {
    let mut overlay = opened();
    let now = Instant::now();
    overlay.handle_key(press(KeyCode::Char('c')), now);
    overlay.handle_key(press(KeyCode::Backspace), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Char('c')), now),
        MmlOverlayAction::Send(vec![[0x80, 60, 0], [0x90, 60, 127]])
    );
}
