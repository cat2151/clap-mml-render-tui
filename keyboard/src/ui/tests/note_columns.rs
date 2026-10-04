use std::time::{Duration, Instant};

use super::*;
use crate::periodic_timeline::LOOKAHEAD;
use crate::NotePlaybackMode;

pub(super) const TICK: Duration = Duration::from_millis(250);

pub(super) fn render_at(state: KeyboardState, now: Instant) -> Terminal<TestBackend> {
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
        .draw(|f| draw(&mut screen, &ready, now, f))
        .unwrap();
    terminal
}

/// `prefix` で始まる行の、ラベルより後のセル列を空白区切りのセル（文字列, 先頭セルの前景色）で返す。
pub(super) fn cells(terminal: &Terminal<TestBackend>, prefix: &str) -> Vec<(String, Color)> {
    let buffer = terminal.backend().buffer();
    let y = (0..buffer.area.height)
        .find(|&y| {
            (0..buffer.area.width)
                .map(|x| buffer.cell((x, y)).unwrap().symbol())
                .collect::<String>()
                .starts_with(prefix)
        })
        .unwrap_or_else(|| panic!("{prefix} not on screen"));
    let start = prefix.chars().count() as u16;
    let mut result: Vec<(String, Color)> = Vec::new();
    let mut current: Option<(String, Color)> = None;
    // keyboard pane の内側（右枠の手前）まで読む。
    for x in start..buffer.area.width {
        let cell = buffer.cell((x, y)).unwrap();
        if cell.symbol() == "│" {
            break;
        }
        if cell.symbol() == " " {
            result.extend(current.take());
        } else {
            let entry = current.get_or_insert_with(|| (String::new(), cell.fg));
            entry.0.push_str(cell.symbol());
        }
    }
    result.extend(current);
    result
}

fn note_cells(terminal: &Terminal<TestBackend>) -> Vec<(String, Color)> {
    cells(terminal, "│Note:    ")
}

pub(super) fn names(cells: &[(String, Color)]) -> Vec<&str> {
    cells.iter().map(|(name, _)| name.as_str()).collect()
}

pub(super) fn lit(cells: &[(String, Color)]) -> Vec<&str> {
    cells
        .iter()
        .filter(|(_, color)| *color == MONOKAI_GREEN)
        .map(|(name, _)| name.as_str())
        .collect()
}

pub(super) fn chord_state(chords: Vec<Vec<u8>>, now: Instant) -> KeyboardState {
    let mut state = KeyboardState::default();
    let _ = state.replace_repeat_chords(chords, now, false);
    state
}

pub(super) fn cycle_to(state: &mut KeyboardState, mode: NotePlaybackMode, now: Instant) {
    while state.note_playback_mode() != mode {
        let _ = state.cycle_note_playback(now);
    }
}

/// pump と同じく LOOKAHEAD 先までの tick を 1 つ進め、その deadline を返す。
pub(super) fn pump(state: &mut KeyboardState, now: Instant) -> Instant {
    state
        .poll_periodic_tick(now + LOOKAHEAD)
        .expect("a tick is due")
        .at
}

/// 和音 C4 G4 の arp を、G5（arp 列の 4 音目）が鳴る時刻まで進める。
pub(super) fn arp_at_g5(t0: Instant) -> (KeyboardState, Instant) {
    let mut state = chord_state(vec![vec![60, 67]], t0);
    cycle_to(&mut state, NotePlaybackMode::Arp, t0);
    let mut deadline = t0;
    for step in 1..=3u32 {
        deadline = pump(&mut state, t0 + TICK * step - LOOKAHEAD);
    }
    (state, deadline)
}

const ARP_COLUMNS: [&str; 9] = ["C4", "D4", "E4", "F4", "G4", "A4", "B4", "C5", "G5"];

#[test]
fn the_arp_upper_octave_gets_columns_and_only_the_sounding_note_is_lit() {
    let (state, deadline) = arp_at_g5(Instant::now());

    let terminal = render_at(state, deadline);

    let cells = note_cells(&terminal);
    assert_eq!(names(&cells), ARP_COLUMNS);
    assert_eq!(lit(&cells), ["G5"]);
}

#[test]
fn the_lit_note_follows_the_real_sound_time_not_the_lookahead() {
    let (state, deadline) = arp_at_g5(Instant::now());

    let terminal = render_at(state, deadline - Duration::from_millis(1));

    assert_eq!(lit(&note_cells(&terminal)), ["C5"]);
}

#[test]
fn advancing_the_arp_keeps_the_columns() {
    let t0 = Instant::now();
    let (mut state, deadline) = arp_at_g5(t0);
    let next = pump(&mut state, deadline + TICK - LOOKAHEAD);

    let terminal = render_at(state, next);

    let cells = note_cells(&terminal);
    assert_eq!(names(&cells), ARP_COLUMNS);
    assert_eq!(lit(&cells), ["C4"]);
}

#[test]
fn repeat_lights_every_note_of_the_sounding_chord() {
    let t0 = Instant::now();
    let mut state = chord_state(vec![vec![60, 64, 67]], t0);
    cycle_to(&mut state, NotePlaybackMode::Repeat, t0);

    let terminal = render_at(state, t0);

    let cells = note_cells(&terminal);
    assert_eq!(names(&cells), ["C4", "D4", "E4", "F4", "G4", "A4", "B4"]);
    assert_eq!(lit(&cells), ["C4", "E4", "G4"]);
}

#[test]
fn a_black_key_from_mml_gets_its_own_column_between_neighbors() {
    let t0 = Instant::now();
    let state = chord_state(vec![vec![61, 65]], t0);

    let terminal = render_at(state, t0);

    let cells = note_cells(&terminal);
    assert_eq!(
        names(&cells),
        ["C4", "C#4", "D4", "E4", "F4", "G4", "A4", "B4"]
    );
    assert!(lit(&cells).is_empty(), "{cells:?}");
    // 最長の音名 `C#4` + 1 で列幅が揃う。
    let row = super::buffer_to_string(&terminal);
    assert!(row.contains("│Note:    C4  C#4 D4  E4  "), "{row}");
}

#[test]
fn without_a_chord_the_note_row_is_the_seven_pc_keys_unlit() {
    let terminal = render_at(KeyboardState::default(), Instant::now());

    let cells = note_cells(&terminal);
    assert_eq!(names(&cells), ["C4", "D4", "E4", "F4", "G4", "A4", "B4"]);
    assert!(lit(&cells).is_empty(), "{cells:?}");
    let screen = super::buffer_to_string(&terminal);
    assert!(
        screen.contains("│PC key:  c   d   e   f   g   a   b "),
        "{screen}"
    );
    assert!(
        screen.contains("│Note:    C4  D4  E4  F4  G4  A4  B4 "),
        "{screen}"
    );
}

#[test]
fn a_held_pc_key_lights_its_key_and_its_note() {
    let mut state = KeyboardState::default();
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());

    let terminal = render_at(state, Instant::now());

    assert_eq!(lit(&note_cells(&terminal)), ["C4"]);
    let keys = cells(&terminal, "│PC key:  ");
    assert_eq!(names(&keys), ["c", "d", "e", "f", "g", "a", "b"]);
    assert_eq!(lit(&keys), ["c"]);
}

#[test]
fn pc_keys_stay_above_their_notes_when_arp_columns_are_added() {
    let (state, deadline) = arp_at_g5(Instant::now());

    let terminal = render_at(state, deadline);

    let screen = super::buffer_to_string(&terminal);
    assert!(
        screen.contains("│PC key:  c   d   e   f   g   a   b           "),
        "{screen}"
    );
}
