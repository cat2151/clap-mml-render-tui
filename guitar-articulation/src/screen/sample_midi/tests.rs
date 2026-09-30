use std::path::PathBuf;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::Take;
use super::*;
use crate::Rule;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn at(seconds: f64, message: [u8; 3]) -> TimedMidiEvent {
    TimedMidiEvent { seconds, message }
}

/// KS 17 と 3 つの演奏音（3 まとまり）。
fn midi_events() -> Vec<TimedMidiEvent> {
    vec![
        at(0.0, [0xB0, 20, 10]),
        at(0.0, [0x90, 17, 100]),
        at(0.0, [0x90, 60, 100]),
        at(0.5, [0x80, 60, 0]),
        at(0.5, [0x90, 62, 100]),
        at(1.0, [0x80, 62, 0]),
        at(1.0, [0x90, 64, 100]),
        at(1.5, [0x80, 64, 0]),
        at(1.5, [0x80, 17, 0]),
    ]
}

fn files() -> Vec<PathBuf> {
    ["a.mid", "b.mid", "c.mid"]
        .iter()
        .map(|name| PathBuf::from("dir").join(name))
        .collect()
}

/// `i` → 文字 → `Enter` で MML を確定した画面。
fn screen_with_mml(mml: &str) -> GuitarArticulationScreen {
    let mut screen = GuitarArticulationScreen::default();
    screen.handle_key_event(key(KeyCode::Char('i')));
    for ch in mml.chars() {
        screen.handle_key_event(key(KeyCode::Char(ch)));
    }
    screen.handle_key_event(key(KeyCode::Enter));
    screen
}

/// MML を確定したうえで、MID モードに入った画面。
fn screen_in_midi_mode() -> GuitarArticulationScreen {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(key(KeyCode::Char('o')));
    screen.open_sample_midi_list(Ok(files()));
    screen.load_sample_midi("b.mid".to_string(), Ok(midi_events()));
    screen
}

#[test]
fn o_asks_for_the_list_in_the_matrix_and_types_in_the_mml_input() {
    let mut screen = screen_with_mml("o3 e");
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('o'))),
        GuitarArticulationAction::OpenSampleMidiList
    );

    screen.handle_key_event(key(KeyCode::Char('i')));
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('o'))),
        GuitarArticulationAction::Continue
    );
    screen.handle_key_event(key(KeyCode::Enter));
    assert_eq!(screen.mml(), "o3 eo");
}

#[test]
fn the_list_stops_at_its_end_and_enter_asks_for_the_selected_file() {
    let mut screen = screen_with_mml("o3 e");
    screen.open_sample_midi_list(Ok(files()));
    assert_eq!(screen.sample_midi_list().unwrap().1, 0);

    for _ in 0..3 {
        assert_eq!(
            screen.handle_key_event(key(KeyCode::Char('j'))),
            GuitarArticulationAction::Continue
        );
    }
    assert_eq!(screen.sample_midi_list().unwrap().1, 2);
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Enter)),
        GuitarArticulationAction::LoadSampleMidi(files()[2].clone())
    );
    screen.handle_key_event(key(KeyCode::Char('k')));
    assert_eq!(screen.sample_midi_list().unwrap().1, 1);
}

#[test]
fn esc_closes_the_list_without_touching_the_state() {
    let mut screen = screen_with_mml("o3 l8 e g");
    let rules = screen.rules().clone();
    screen.open_sample_midi_list(Ok(files()));
    screen.handle_key_event(key(KeyCode::Char('a')));
    screen.handle_key_event(key(KeyCode::Esc));

    assert!(screen.sample_midi_list().is_none());
    assert!(screen.sample_midi().is_none());
    assert_eq!(screen.rules(), &rules);
    assert_eq!(screen.mml(), "o3 l8 e g");
}

#[test]
fn an_unreadable_or_empty_folder_does_not_open_the_list() {
    let mut screen = screen_with_mml("o3 e");
    screen.open_sample_midi_list(Err("置き場がありません: x".to_string()));
    assert!(screen.sample_midi_list().is_none());
    assert_eq!(screen.error.as_deref(), Some("置き場がありません: x"));

    screen.open_sample_midi_list(Ok(Vec::new()));
    assert!(screen.sample_midi_list().is_none());
    assert!(screen.error.is_some());
}

#[test]
fn a_file_that_fails_to_load_keeps_the_list_open_with_the_reason() {
    let mut screen = screen_with_mml("o3 e");
    screen.open_sample_midi_list(Ok(files()));
    screen.load_sample_midi("a.mid".to_string(), Err("壊れています".to_string()));

    assert!(screen.sample_midi_list().is_some());
    assert!(screen.sample_midi().is_none());
    assert!(screen.error.as_deref().unwrap().contains("壊れています"));
}

#[test]
fn loading_enters_the_midi_mode_and_space_plays_the_whole_file() {
    let mut screen = screen_in_midi_mode();

    assert!(screen.sample_midi_list().is_none());
    assert_eq!(screen.sample_midi().unwrap().name(), "b.mid");
    assert_eq!(screen.sample_midi().unwrap().group_count(), 3);
    assert_eq!(screen.sample_midi_cursor(), 0);
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char(' '))),
        GuitarArticulationAction::PlaySampleMidi { note: None }
    );
    assert_eq!(screen.sample_midi_events(None), midi_events());
}

#[test]
fn the_note_preview_plays_one_note_and_h_l_move_between_notes() {
    let mut screen = screen_in_midi_mode();

    // n OFF の移動は鳴らさない。
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('l'))),
        GuitarArticulationAction::Continue
    );
    screen.handle_key_event(key(KeyCode::Char('h')));
    assert_eq!(screen.sample_midi_cursor(), 0);

    screen.handle_key_event(key(KeyCode::Char('n')));
    assert!(screen.note_preview());
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char(' '))),
        GuitarArticulationAction::PlaySampleMidi { note: Some(0) }
    );
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('l'))),
        GuitarArticulationAction::PlaySampleMidi { note: Some(1) }
    );
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Right)),
        GuitarArticulationAction::PlaySampleMidi { note: Some(2) }
    );
    // 最後のまとまりでは動かず鳴らさない。
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('l'))),
        GuitarArticulationAction::Continue
    );
    assert_eq!(screen.sample_midi_cursor(), 2);
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Left)),
        GuitarArticulationAction::PlaySampleMidi { note: Some(1) }
    );
    assert_eq!(
        screen.sample_midi_events(Some(1)),
        screen.sample_midi().unwrap().note_events(1)
    );
}

#[test]
fn the_mml_side_keys_do_nothing_in_the_midi_mode() {
    let mut screen = screen_in_midi_mode();
    let rules = screen.rules().clone();

    for code in [
        KeyCode::Char('a'),
        KeyCode::Char('b'),
        KeyCode::Char('x'),
        KeyCode::Char('i'),
    ] {
        assert_eq!(
            screen.handle_key_event(key(code)),
            GuitarArticulationAction::Continue,
            "{code:?}"
        );
    }
    assert_eq!(
        screen.handle_key_event(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT)),
        GuitarArticulationAction::Continue
    );
    assert!(!screen.input_open());
    assert_eq!(screen.rules(), &rules);
    assert_eq!(screen.mml(), "o3 l8 e g a");
    assert!(screen.sample_midi().is_some());
}

#[test]
fn o_reopens_the_list_on_the_current_file() {
    let mut screen = screen_in_midi_mode();
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('o'))),
        GuitarArticulationAction::OpenSampleMidiList
    );
    screen.open_sample_midi_list(Ok(files()));
    assert_eq!(screen.sample_midi_list().unwrap().1, 1);
}

#[test]
fn esc_leaves_the_midi_mode_with_the_mml_side_as_it_was() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('a')));
    let mml = screen.mml().to_string();
    let rules = screen.rules().clone();
    let cursor = screen.cursor();
    assert!(rules.is_on(1, Rule::HammerPull));

    screen.open_sample_midi_list(Ok(files()));
    screen.load_sample_midi("b.mid".to_string(), Ok(midi_events()));
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Esc));

    assert!(screen.sample_midi().is_none());
    assert_eq!(screen.mml(), mml);
    assert_eq!(screen.rules(), &rules);
    assert_eq!(screen.cursor(), cursor);
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char(' '))),
        GuitarArticulationAction::Play(Take::Converted)
    );
}

#[test]
fn entering_the_screen_again_keeps_the_midi_mode_without_opening_the_input() {
    let mut screen = screen_in_midi_mode();
    assert_eq!(screen.enter(), GuitarArticulationAction::Continue);
    assert!(!screen.input_open());
    assert!(screen.sample_midi().is_some());
}
