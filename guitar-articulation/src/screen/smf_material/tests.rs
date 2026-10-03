use crossterm::event::KeyModifiers;

use super::*;
use crate::ui::RULE_ROWS;
use crate::{RowRule, Rule};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn shift(ch: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(ch), KeyModifiers::SHIFT)
}

fn at(seconds: f64, message: [u8; 3]) -> TimedMidiEvent {
    TimedMidiEvent { seconds, message }
}

/// 和音 C4/E4/G4（0〜1）と、メロディ A4（1〜2）。列は 2 つ。
fn chord_then_melody() -> Vec<TimedMidiEvent> {
    vec![
        at(0.0, [0x90, 60, 100]),
        at(0.0, [0x90, 64, 100]),
        at(0.0, [0x90, 67, 100]),
        at(1.0, [0x80, 60, 0]),
        at(1.0, [0x80, 64, 0]),
        at(1.0, [0x80, 67, 0]),
        at(1.0, [0x90, 69, 100]),
        at(2.0, [0x80, 69, 0]),
    ]
}

fn top_notes_of_chord_then_melody() -> Vec<TimedMidiEvent> {
    vec![
        at(0.0, [0x90, 67, 100]),
        at(1.0, [0x80, 67, 0]),
        at(1.0, [0x90, 69, 100]),
        at(2.0, [0x80, 69, 0]),
    ]
}

fn type_text(screen: &mut GuitarArticulationScreen, text: &str) {
    for ch in text.chars() {
        screen.handle_key_event(key(KeyCode::Char(ch)));
    }
}

/// `i` → 文字 → `Enter` で MML を確定した画面。
fn screen_with_mml(mml: &str) -> GuitarArticulationScreen {
    let mut screen = GuitarArticulationScreen::default();
    screen.handle_key_event(key(KeyCode::Char('i')));
    type_text(&mut screen, mml);
    screen.handle_key_event(key(KeyCode::Enter));
    screen
}

/// MML を確定したうえで、`O` から SMF を読んだ画面。
fn screen_with_smf() -> GuitarArticulationScreen {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(shift('O'));
    let action = screen.load_smf(
        PathBuf::from("dir").join("song.mid"),
        Ok(chord_then_melody()),
    );
    assert_eq!(action, GuitarArticulationAction::Play(Take::Converted));
    screen
}

fn column_rule_key(rule: Rule) -> char {
    RULE_ROWS.iter().find(|row| row.rule == rule).unwrap().key
}

#[test]
fn shift_o_opens_the_input_and_enter_asks_for_the_trimmed_path() {
    let mut screen = screen_with_mml("o3 e");

    screen.handle_key_event(shift('O'));
    assert!(screen.smf_input_open());
    assert!(screen.uses_textarea_cursor());

    // 英字は入力欄へ入り、ルールや演奏にはならない。
    type_text(&mut screen, "  \"C:\\midi\\b a.mid\" ");
    let action = screen.handle_key_event(key(KeyCode::Enter));

    assert_eq!(
        action,
        GuitarArticulationAction::LoadSmf(PathBuf::from("C:\\midi\\b a.mid"))
    );
    // 読み込み結果が来るまで開いたまま。
    assert!(screen.smf_input_open());
    assert!(screen.rules().is_empty());
}

#[test]
fn enter_with_an_empty_path_shows_an_error_and_keeps_the_input() {
    let mut screen = screen_with_mml("o3 e");
    screen.handle_key_event(shift('O'));
    type_text(&mut screen, " \"\" ");

    let action = screen.handle_key_event(key(KeyCode::Enter));

    assert_eq!(action, GuitarArticulationAction::Continue);
    assert!(screen.error.is_some());
    assert!(screen.smf_input_open());
}

#[test]
fn esc_closes_the_input_without_loading() {
    let mut screen = screen_with_mml("o3 e");
    screen.handle_key_event(shift('O'));
    type_text(&mut screen, "a.mid");

    screen.handle_key_event(key(KeyCode::Esc));

    assert!(!screen.smf_input_open());
    assert_eq!(screen.smf_material_name(), None);
    assert_eq!(screen.mml(), "o3 e");
}

#[test]
fn ctrl_m_also_commits_the_path() {
    let mut screen = screen_with_mml("o3 e");
    screen.handle_key_event(shift('O'));
    type_text(&mut screen, "a.mid");

    let action = screen.handle_key_event(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::CONTROL));

    assert_eq!(
        action,
        GuitarArticulationAction::LoadSmf(PathBuf::from("a.mid"))
    );
}

#[test]
fn loading_makes_the_smf_the_material_and_keeps_the_mml() {
    let screen = screen_with_smf();

    assert!(!screen.smf_input_open());
    assert_eq!(screen.smf_material_name(), Some("song.mid"));
    assert_eq!(
        screen.smf_material_path(),
        Some(PathBuf::from("dir").join("song.mid").as_path())
    );
    assert_eq!(screen.events(Take::Plain), chord_then_melody().as_slice());
    assert_eq!(screen.column_count(), 2);
    assert_eq!(screen.notes().len(), 4);
    assert!(!screen.events(Take::Converted).is_empty());
    assert_eq!(screen.mml(), "o3 l8 e g a");
    assert_eq!(screen.cursor(), 0);
    assert_eq!(screen.error, None);
}

#[test]
fn a_failed_load_keeps_the_input_open_with_the_reason() {
    let mut screen = screen_with_mml("o3 e");
    screen.handle_key_event(shift('O'));
    type_text(&mut screen, "missing.mid");

    let action = screen.load_smf(
        PathBuf::from("dir").join("missing.mid"),
        Err("見つかりません".to_string()),
    );

    assert_eq!(action, GuitarArticulationAction::Continue);
    assert!(screen.smf_input_open());
    assert_eq!(screen.error.as_deref(), Some("missing.mid: 見つかりません"));
    assert_eq!(screen.smf_material_name(), None);
    assert_eq!(screen.mml(), "o3 e");
}

#[test]
fn the_input_starts_with_the_last_loaded_path() {
    let mut screen = screen_with_smf();

    screen.handle_key_event(shift('O'));
    let action = screen.handle_key_event(key(KeyCode::Enter));

    assert_eq!(
        action,
        GuitarArticulationAction::LoadSmf(PathBuf::from("dir").join("song.mid"))
    );
}

#[test]
fn loading_clears_column_rules_and_keeps_row_rules() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(key(KeyCode::Char(column_rule_key(Rule::PalmMute))));
    screen.handle_key_event(key(KeyCode::Char('d')));
    assert!(screen.rules().is_on(0, Rule::PalmMute));
    assert!(screen.rules().is_row_on(RowRule::Humanize));

    screen.load_smf(PathBuf::from("song.mid"), Ok(chord_then_melody()));

    assert!(!screen.rules().is_on(0, Rule::PalmMute));
    assert!(screen.rules().is_row_on(RowRule::Humanize));
}

#[test]
fn shift_m_keeps_only_the_top_notes_and_toggles_back() {
    let mut screen = screen_with_smf();
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char(column_rule_key(Rule::PalmMute))));
    assert!(screen.rules().is_on(1, Rule::PalmMute));

    let action = screen.handle_key_event(shift('M'));

    assert_eq!(action, GuitarArticulationAction::Play(Take::Converted));
    assert!(screen.smf_top_note());
    assert_eq!(
        screen.events(Take::Plain),
        top_notes_of_chord_then_melody().as_slice()
    );
    assert_eq!(screen.column_count(), 2);
    assert_eq!(screen.notes().len(), 2);
    assert_eq!(screen.cursor(), 0);
    assert!(!screen.rules().is_on(1, Rule::PalmMute));

    screen.handle_key_event(shift('M'));

    assert!(!screen.smf_top_note());
    assert_eq!(screen.events(Take::Plain), chord_then_melody().as_slice());
}

#[test]
fn top_note_stays_on_for_the_next_load() {
    let mut screen = screen_with_smf();
    screen.handle_key_event(shift('M'));

    screen.load_smf(PathBuf::from("other.mid"), Ok(chord_then_melody()));

    assert!(screen.smf_top_note());
    assert_eq!(
        screen.events(Take::Plain),
        top_notes_of_chord_then_melody().as_slice()
    );
}

#[test]
fn shift_m_without_smf_shows_an_error() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    let plain = screen.events(Take::Plain).to_vec();

    let action = screen.handle_key_event(shift('M'));

    assert_eq!(action, GuitarArticulationAction::Continue);
    assert_eq!(screen.error.as_deref(), Some("O で SMF を読んでください"));
    assert!(!screen.smf_top_note());
    assert_eq!(screen.events(Take::Plain), plain.as_slice());
}

#[test]
fn existing_keys_play_the_smf_material() {
    let mut screen = screen_with_smf();

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('b'))),
        GuitarArticulationAction::Play(Take::Plain)
    );
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char(' '))),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('l'))),
        GuitarArticulationAction::PlayNote {
            take: Take::Converted,
            column: 1
        }
    );
    // 和音の列は 3 音とも鳴る。
    screen.handle_key_event(key(KeyCode::Char('h')));
    let on_count = screen
        .column_events(Take::Plain)
        .iter()
        .filter(|event| event.message[0] & 0xF0 == 0x90)
        .count();
    assert_eq!(on_count, 3);

    screen.handle_key_event(key(KeyCode::Char('n')));
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('b'))),
        GuitarArticulationAction::PlayNote {
            take: Take::Plain,
            column: 0
        }
    );
}

#[test]
fn rule_toggles_apply_but_do_not_record_history() {
    let mut screen = screen_with_smf();
    let history_len = screen.history().entries.len();

    let action = screen.handle_key_event(key(KeyCode::Char(column_rule_key(Rule::PalmMute))));
    screen.handle_key_event(key(KeyCode::Char('d')));
    screen.handle_key_event(shift('A'));

    assert_eq!(action, GuitarArticulationAction::Play(Take::Converted));
    assert!(screen.rules().is_on(0, Rule::PalmMute));
    assert!(screen.rules().is_row_on(RowRule::Humanize));
    assert_eq!(screen.history().entries.len(), history_len);
}

#[test]
fn z_and_shift_h_do_not_open_while_the_smf_is_the_material() {
    let mut screen = screen_with_smf();
    let history_len = screen.history().entries.len();

    let action = screen.handle_key_event(key(KeyCode::Char('z')));
    assert_eq!(action, GuitarArticulationAction::Continue);
    assert!(screen.error.is_some());
    assert!(!screen.repeat());

    let action = screen.handle_key_event(shift('H'));
    assert_eq!(action, GuitarArticulationAction::Continue);
    assert!(screen.error.is_some());
    assert_eq!(screen.history_overlay_selected(), None);

    assert_eq!(screen.smf_material_name(), Some("song.mid"));
    assert_eq!(screen.events(Take::Plain), chord_then_melody().as_slice());
    assert_eq!(screen.history().entries.len(), history_len);
}

#[test]
fn enter_does_not_open_the_mml_input_while_the_smf_is_the_material() {
    let mut screen = screen_with_smf();

    assert_eq!(screen.enter(), GuitarArticulationAction::Continue);
    assert!(!screen.input_open());
    assert_eq!(screen.events(Take::Plain), chord_then_melody().as_slice());
}

#[test]
fn committing_mml_with_i_returns_to_the_mml_material() {
    let mut screen = screen_with_smf();
    let history_len = screen.history().entries.len();

    screen.handle_key_event(key(KeyCode::Char('i')));
    screen.handle_key_event(key(KeyCode::End));
    type_text(&mut screen, " b");
    screen.handle_key_event(key(KeyCode::Enter));

    assert_eq!(screen.smf_material_name(), None);
    let expected = cmrt_chord::timed_performance("o3 l8 e g a b")
        .unwrap()
        .events;
    assert_eq!(screen.events(Take::Plain), expected.as_slice());
    assert_eq!(screen.column_count(), 4);
    // MML 素材へ戻った確定は履歴へ積む。
    assert_eq!(screen.history().entries.len(), history_len + 1);
}

#[test]
fn esc_from_the_mml_input_keeps_the_smf_material() {
    let mut screen = screen_with_smf();

    screen.handle_key_event(key(KeyCode::Char('i')));
    screen.handle_key_event(key(KeyCode::Esc));

    assert_eq!(screen.smf_material_name(), Some("song.mid"));
    assert_eq!(screen.events(Take::Plain), chord_then_melody().as_slice());
}

#[test]
fn column_rules_toggled_on_the_smf_do_not_move_the_mml_anchor() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char(column_rule_key(Rule::PalmMute))));
    screen.load_smf(PathBuf::from("song.mid"), Ok(chord_then_melody()));
    screen.handle_key_event(key(KeyCode::Char(column_rule_key(Rule::PalmMute))));

    screen.handle_key_event(key(KeyCode::Char('i')));
    screen.handle_key_event(key(KeyCode::Enter));

    // MML へ戻ると、MML で付けた列ルールが戻り、SMF で付けたものは残らない。
    assert!(screen.rules().is_on(1, Rule::PalmMute));
    assert!(!screen.rules().is_on(0, Rule::PalmMute));
}
