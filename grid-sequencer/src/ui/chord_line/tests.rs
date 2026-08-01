use std::time::Instant;

use cmrt_tui_core::theme::{MONOKAI_GRAY, MONOKAI_GREEN};

use super::*;
use crate::ChordPlayback;

fn playback() -> ChordPlayback {
    ChordPlayback::new(
        "C#",
        "I-IV-V-I".to_string(),
        vec![
            vec![61, 65, 68],
            vec![66, 70, 73],
            vec![68, 72, 75],
            vec![61, 65, 68],
        ],
    )
    .unwrap()
}

fn screen_with_chord() -> GridSequencerScreen {
    let mut screen = GridSequencerScreen::new(None);
    screen.state.set_chord(Some(playback()), Instant::now());
    screen
}

fn text_of(line: &Line<'static>) -> String {
    line.spans
        .iter()
        .map(|span| span.content.as_ref())
        .collect()
}

#[test]
fn the_line_is_absent_while_the_chord_mode_is_off() {
    assert!(line(&GridSequencerScreen::new(None)).is_none());
}

#[test]
fn the_line_shows_the_key_progression_and_position() {
    let line = line(&screen_with_chord()).unwrap();

    assert_eq!(text_of(&line), " chord Key:C#  I-IV-V-I  [1/4] ");
}

#[test]
fn only_the_chord_being_played_is_highlighted() {
    let screen = screen_with_chord();

    let line = line(&screen).unwrap();

    let highlighted = line
        .spans
        .iter()
        .filter(|span| span.style.fg == Some(MONOKAI_GREEN))
        .map(|span| span.content.to_string())
        .collect::<Vec<_>>();
    assert_eq!(highlighted, vec!["I".to_string()]);
}

/// 進行の記法が変わって分割数が合わなくなったら、位置を偽らずに素の文字列を出す。
#[test]
fn a_progression_that_cannot_be_split_per_chord_is_shown_as_is() {
    let mut screen = GridSequencerScreen::new(None);
    let playback = ChordPlayback::new(
        "C",
        "I IV V".to_string(),
        vec![vec![60, 64, 67], vec![65, 69, 72], vec![67, 71, 74]],
    )
    .unwrap();
    screen.state.set_chord(Some(playback), Instant::now());

    let line = line(&screen).unwrap();

    assert_eq!(text_of(&line), " chord Key:C  I IV V  [1/3] ");
    assert!(line
        .spans
        .iter()
        .all(|span| span.style.fg == Some(MONOKAI_GRAY)));
}

#[test]
fn the_reason_the_chord_mode_could_not_start_replaces_the_progression() {
    let mut screen = GridSequencerScreen::new(None);
    screen.chord_error = Some("poly patch が見つかりません".to_string());

    let line = line(&screen).unwrap();

    assert_eq!(text_of(&line), " chord: poly patch が見つかりません ");
}
