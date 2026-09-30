use super::{key, screen_with_mml};
use crate::{GuitarArticulationAction, RowRule, Take};
use crossterm::event::KeyCode;

#[test]
fn d_toggles_humanize_for_the_whole_row_and_plays_the_new_take() {
    let mut screen = screen_with_mml("o3 l16 e f+ g a b a g f+");
    let plain = screen.events(Take::Plain).to_vec();
    let converted = screen.events(Take::Converted).to_vec();

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('d'))),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert!(screen.rules().is_row_on(RowRule::Humanize));
    assert_ne!(screen.events(Take::Converted), converted.as_slice());
    assert_eq!(screen.events(Take::Plain), plain.as_slice());

    screen.handle_key_event(key(KeyCode::Char('d')));
    assert!(!screen.rules().is_row_on(RowRule::Humanize));
    assert_eq!(screen.events(Take::Converted), converted.as_slice());
}

#[test]
fn the_played_column_start_follows_humanize_only_in_the_converted_take() {
    let mut screen = screen_with_mml("o3 l16 e f+ g a b a g f+");
    let written: Vec<Option<f64>> = (0..screen.column_count())
        .map(|column| screen.column_on_seconds(column, Take::Plain))
        .collect();
    let off: Vec<Option<f64>> = (0..screen.column_count())
        .map(|column| screen.column_on_seconds(column, Take::Converted))
        .collect();
    assert_eq!(off, written);

    screen.handle_key_event(key(KeyCode::Char('d')));
    let plain: Vec<Option<f64>> = (0..screen.column_count())
        .map(|column| screen.column_on_seconds(column, Take::Plain))
        .collect();
    let on: Vec<Option<f64>> = (0..screen.column_count())
        .map(|column| screen.column_on_seconds(column, Take::Converted))
        .collect();
    assert_eq!(plain, written);
    assert_ne!(on, written);
    // ずらした後の秒は、Articulated の演奏音の note on に実在する。
    for seconds in on.into_iter().flatten() {
        assert!(
            screen
                .events(Take::Converted)
                .iter()
                .any(|e| e.message[0] & 0xF0 == 0x90 && e.message[2] != 0 && e.seconds == seconds),
            "{seconds}"
        );
    }
}

#[test]
fn r_toggles_humanize_release_for_the_whole_row_and_plays_the_new_take() {
    let mut screen = screen_with_mml("o3 l16 e f+ g a b a g f+");
    let plain = screen.events(Take::Plain).to_vec();
    let converted = screen.events(Take::Converted).to_vec();

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('r'))),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert!(screen.rules().is_row_on(RowRule::HumanizeRelease));
    assert_ne!(screen.events(Take::Converted), converted.as_slice());
    assert_eq!(screen.events(Take::Plain), plain.as_slice());

    screen.handle_key_event(key(KeyCode::Char('r')));
    assert!(!screen.rules().is_row_on(RowRule::HumanizeRelease));
    assert_eq!(screen.events(Take::Converted), converted.as_slice());
}

#[test]
fn r_and_d_toggle_independently() {
    let mut screen = screen_with_mml("o3 l16 e f+ g a b a g f+");
    let on = |screen: &crate::GuitarArticulationScreen| {
        (
            screen.rules().is_row_on(RowRule::Humanize),
            screen.rules().is_row_on(RowRule::HumanizeRelease),
        )
    };
    let mut takes = vec![screen.events(Take::Converted).to_vec()];

    screen.handle_key_event(key(KeyCode::Char('d')));
    assert_eq!(on(&screen), (true, false));
    takes.push(screen.events(Take::Converted).to_vec());

    screen.handle_key_event(key(KeyCode::Char('r')));
    assert_eq!(on(&screen), (true, true));
    takes.push(screen.events(Take::Converted).to_vec());

    screen.handle_key_event(key(KeyCode::Char('d')));
    assert_eq!(on(&screen), (false, true));
    takes.push(screen.events(Take::Converted).to_vec());

    screen.handle_key_event(key(KeyCode::Char('r')));
    assert_eq!(on(&screen), (false, false));
    assert_eq!(screen.events(Take::Converted), takes[0].as_slice());

    // なし / d / d+r / r の 4 つの演奏が互いに違う。
    for i in 0..takes.len() {
        for j in i + 1..takes.len() {
            assert_ne!(takes[i], takes[j], "{i} と {j}");
        }
    }
}
