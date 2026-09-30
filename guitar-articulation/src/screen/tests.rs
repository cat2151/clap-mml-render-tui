use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::*;
use crate::KEYSWITCH_VELOCITY;

mod articulation_keys;
mod column_rule_follow;
mod humanize_keys;
mod startup_instrument_keys;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
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

#[test]
fn committing_mml_builds_the_plain_and_converted_events() {
    let screen = screen_with_mml("o3 l8 e f+ g");

    assert!(!screen.input_open());
    assert_eq!(screen.mml(), "o3 l8 e f+ g");
    assert_eq!(screen.column_count(), 3);
    let expected = cmrt_chord::timed_performance("o3 l8 e f+ g")
        .unwrap()
        .events;
    assert_eq!(screen.events(Take::Plain), expected.as_slice());
    assert_eq!(
        screen.events(Take::Converted),
        convert(&expected, &RuleTable::default()).as_slice()
    );
    assert_eq!(
        screen.events(Take::Converted)[0].message,
        [0x90, 17, KEYSWITCH_VELOCITY]
    );
}

#[test]
fn letters_go_to_the_input_while_editing() {
    let screen = screen_with_mml("b");

    // `b` が演奏ではなく入力欄の文字として入ったこと。
    assert_eq!(screen.mml(), "b");
}

#[test]
fn esc_discards_the_edit() {
    let mut screen = screen_with_mml("o3 e");
    screen.handle_key_event(key(KeyCode::Char('i')));
    screen.handle_key_event(key(KeyCode::Char('g')));
    screen.handle_key_event(key(KeyCode::Esc));

    assert!(!screen.input_open());
    assert_eq!(screen.mml(), "o3 e");
}

#[test]
fn an_empty_mml_clears_everything() {
    let mut screen = screen_with_mml("o3 e");
    screen.handle_key_event(key(KeyCode::Char('i')));
    for _ in 0.."o3 e".len() {
        screen.handle_key_event(key(KeyCode::Backspace));
    }
    screen.handle_key_event(key(KeyCode::Enter));

    assert!(!screen.input_open());
    assert_eq!(screen.column_count(), 0);
    assert!(screen.events(Take::Plain).is_empty());
    assert!(screen.events(Take::Converted).is_empty());
}

#[test]
fn b_and_space_ask_for_the_plain_and_converted_takes() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('b'))),
        GuitarArticulationAction::Play(Take::Plain)
    );
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char(' '))),
        GuitarArticulationAction::Play(Take::Converted)
    );
}

#[test]
fn playing_without_mml_explains_instead_of_playing() {
    let mut screen = GuitarArticulationScreen::default();

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char(' '))),
        GuitarArticulationAction::Continue
    );
    assert!(screen.error.is_some());
}

#[test]
fn q_quits_only_outside_the_input() {
    let mut screen = GuitarArticulationScreen::default();
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('q'))),
        GuitarArticulationAction::Quit
    );
    screen.handle_key_event(key(KeyCode::Char('i')));
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('q'))),
        GuitarArticulationAction::Continue
    );
}

#[test]
fn committing_mml_plays_the_converted_take_and_an_empty_one_does_not() {
    let mut screen = GuitarArticulationScreen::default();
    screen.handle_key_event(key(KeyCode::Char('i')));
    for ch in "o3 e".chars() {
        screen.handle_key_event(key(KeyCode::Char(ch)));
    }
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Enter)),
        GuitarArticulationAction::Play(Take::Converted)
    );

    screen.handle_key_event(key(KeyCode::Char('i')));
    for _ in 0.."o3 e".len() {
        screen.handle_key_event(key(KeyCode::Backspace));
    }
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Enter)),
        GuitarArticulationAction::Continue
    );
}

#[test]
fn h_and_l_move_the_cursor_within_the_columns_without_playing() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('h'))),
        GuitarArticulationAction::Continue
    );
    assert_eq!(screen.cursor(), 0, "左端で止まる");
    for expected in [1, 2, 2] {
        assert_eq!(
            screen.handle_key_event(key(KeyCode::Char('l'))),
            GuitarArticulationAction::Continue
        );
        assert_eq!(screen.cursor(), expected);
    }
    screen.handle_key_event(key(KeyCode::Char('h')));
    assert_eq!(screen.cursor(), 1);
}

#[test]
fn a_toggles_hammer_pull_on_the_cursor_column_and_plays_the_new_take() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    let before = screen.events(Take::Converted).to_vec();

    screen.handle_key_event(key(KeyCode::Char('l')));
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('a'))),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert!(screen.rules().is_on(1, Rule::HammerPull));
    assert!(!screen.rules().is_on(0, Rule::HammerPull));
    let hammer = screen.events(Take::Converted).to_vec();
    assert_ne!(hammer, before);
    assert!(
        hammer.iter().any(|event| event.message == [0x90, 26, 127]),
        "上行なので Hammer-On: {hammer:?}"
    );
    assert_eq!(
        hammer,
        convert(screen.events(Take::Plain), screen.rules()),
        "表示と演奏に使う列はルール表から作り直したもの"
    );

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('a'))),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert!(screen.rules().is_empty());
    assert_eq!(screen.events(Take::Converted), before.as_slice());
}

#[test]
fn toggling_without_mml_explains_instead_of_playing() {
    let mut screen = GuitarArticulationScreen::default();

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('a'))),
        GuitarArticulationAction::Continue
    );
    assert!(screen.error.is_some());
    assert!(screen.rules().is_empty());
}

#[test]
fn recommitting_the_same_mml_keeps_the_column_rules_and_resets_the_cursor() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('a')));

    screen.handle_key_event(key(KeyCode::Char('i')));
    screen.handle_key_event(key(KeyCode::Enter));

    assert!(screen.rules().is_on(1, Rule::HammerPull));
    assert_eq!(screen.rules().column_count_of(Rule::HammerPull), 1);
    assert_eq!(screen.cursor(), 0);
}

#[test]
fn entering_with_an_empty_mml_opens_the_input_and_plays_the_default_mml() {
    let mut screen = GuitarArticulationScreen::default();

    let action = screen.enter();

    assert_eq!(action, GuitarArticulationAction::Play(Take::Converted));
    assert!(screen.input_open());
    assert_eq!(screen.mml(), DEFAULT_MML);
    assert_eq!(screen.column_count(), 8);
    // 開いた欄には既定の MML が入っていて、そのまま Enter で確定できる。
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Enter)),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert_eq!(screen.mml(), DEFAULT_MML);
}

#[test]
fn entering_with_an_mml_only_opens_the_input() {
    let mut screen = screen_with_mml("o3 e");

    assert_eq!(screen.enter(), GuitarArticulationAction::Continue);
    assert!(screen.input_open());
    assert_eq!(screen.mml(), "o3 e");
}

#[test]
fn question_mark_opens_the_help_even_while_typing_and_esc_closes_only_the_help() {
    let mut screen = GuitarArticulationScreen::default();
    screen.enter();

    screen.handle_key_event(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT));
    assert!(screen.help_open());
    screen.handle_key_event(key(KeyCode::Esc));

    assert!(!screen.help_open());
    assert!(screen.input_open(), "Esc は help だけを閉じる");
    assert_eq!(screen.mml(), DEFAULT_MML, "`?` は MML に入らない");
}

#[test]
fn keys_behind_the_help_do_nothing() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    screen.handle_key_event(key(KeyCode::Char('?')));

    for code in [KeyCode::Char('l'), KeyCode::Char('a'), KeyCode::Char(' ')] {
        assert_eq!(
            screen.handle_key_event(key(code)),
            GuitarArticulationAction::Continue
        );
    }
    assert_eq!(screen.cursor(), 0);
    assert!(screen.rules().is_empty());

    screen.handle_key_event(key(KeyCode::Char('?')));
    assert!(!screen.help_open());
}

/// 入力欄は `i` で開く。`Enter` では開かない。
#[test]
fn i_opens_the_input_and_enter_does_not() {
    let mut screen = GuitarArticulationScreen::default();

    screen.handle_key_event(key(KeyCode::Enter));
    assert!(!screen.input_open());

    screen.handle_key_event(key(KeyCode::Char('i')));
    assert!(screen.input_open());
}

#[test]
fn s_toggles_auto_hammer_pull_for_the_whole_row_and_plays_the_new_take() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    let before = screen.events(Take::Converted).to_vec();

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('s'))),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert!(screen.rules().is_row_on(RowRule::AutoHammerPull));
    let auto = screen.events(Take::Converted).to_vec();
    let hammers = auto
        .iter()
        .filter(|event| event.message == [0x90, 26, 127])
        .count();
    assert_eq!(
        hammers, 1,
        "2 列目から Hammer-On、3 列目は KS が同じなので送らない: {auto:?}"
    );

    screen.handle_key_event(key(KeyCode::Char('s')));
    assert!(!screen.rules().is_row_on(RowRule::AutoHammerPull));
    assert_eq!(screen.events(Take::Converted), before.as_slice());
}

#[test]
fn auto_hammer_pull_survives_a_new_mml() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    screen.handle_key_event(key(KeyCode::Char('s')));
    screen.handle_key_event(key(KeyCode::Char('i')));
    screen.handle_key_event(key(KeyCode::Char('a')));
    screen.handle_key_event(key(KeyCode::Enter));

    assert_eq!(screen.mml(), "o3 l8 e f+ ga");
    assert!(screen.rules().is_row_on(RowRule::AutoHammerPull));
    assert!(screen.rules().is_empty());
}

#[test]
fn n_toggles_the_note_preview_without_playing() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    assert!(!screen.note_preview());
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char(' '))),
        GuitarArticulationAction::Play(Take::Converted)
    );

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('n'))),
        GuitarArticulationAction::Continue
    );
    assert!(screen.note_preview());
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char(' '))),
        GuitarArticulationAction::PlayNote {
            take: Take::Converted,
            column: screen.cursor(),
        }
    );

    screen.handle_key_event(key(KeyCode::Char('n')));
    assert!(!screen.note_preview());
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char(' '))),
        GuitarArticulationAction::Play(Take::Converted)
    );
}

#[test]
fn rule_toggles_and_b_play_only_the_cursor_column_in_the_note_preview() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    screen.handle_key_event(key(KeyCode::Char('n')));
    screen.handle_key_event(key(KeyCode::Char('l')));

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('a'))),
        GuitarArticulationAction::PlayNote {
            take: Take::Converted,
            column: 1,
        }
    );
    assert!(screen.rules().is_on(1, Rule::HammerPull));
    for (ch, take) in [
        ('e', Take::Converted),
        ('s', Take::Converted),
        ('b', Take::Plain),
    ] {
        assert_eq!(
            screen.handle_key_event(key(KeyCode::Char(ch))),
            GuitarArticulationAction::PlayNote { take, column: 1 },
            "{ch}"
        );
    }
}

#[test]
fn committing_mml_plays_the_whole_phrase_even_in_the_note_preview() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    screen.handle_key_event(key(KeyCode::Char('n')));
    screen.handle_key_event(key(KeyCode::Char('i')));
    screen.handle_key_event(key(KeyCode::End));
    screen.handle_key_event(key(KeyCode::Char(' ')));
    screen.handle_key_event(key(KeyCode::Char('a')));

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Enter)),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert!(screen.note_preview());
}

#[test]
fn n_goes_to_the_input_while_editing() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    screen.handle_key_event(key(KeyCode::Char('i')));
    screen.handle_key_event(key(KeyCode::End));
    screen.handle_key_event(key(KeyCode::Char('n')));

    assert!(!screen.note_preview());
    assert_eq!(
        cmrt_tui_core::text_input::textarea_value(screen.input().unwrap()),
        "o3 l8 e f+ gn"
    );
}

#[test]
fn column_events_are_the_cursor_column_of_the_articulated_notes() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('a')));

    assert_eq!(
        screen.column_events(Take::Converted),
        crate::column_events(
            screen.notes(),
            screen.articulated(),
            screen.rules(),
            screen.cursor(),
            Take::Converted
        )
    );
    assert!(!screen.column_events(Take::Converted).is_empty());
}
