use super::{key, screen_with_mml};
use crate::ui::RULE_ROWS;
use crate::{GuitarArticulationAction, GuitarArticulationScreen, Rule, Take};
use crossterm::event::KeyCode;

#[test]
fn t_then_j_k_then_enter_toggles_the_selected_rule_on_the_cursor_column() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(key(KeyCode::Char('l')));
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('t'))),
        GuitarArticulationAction::Continue
    );
    assert_eq!(screen.rule_list_selected(), Some(0));

    let portamento = RULE_ROWS
        .iter()
        .position(|(rule, _, _)| *rule == Rule::Portamento)
        .unwrap();
    for _ in 0..portamento {
        screen.handle_key_event(key(KeyCode::Char('j')));
    }
    screen.handle_key_event(key(KeyCode::Char('k')));
    assert_eq!(screen.rule_list_selected(), Some(portamento - 1));
    screen.handle_key_event(key(KeyCode::Down));
    assert_eq!(screen.rule_list_selected(), Some(portamento));
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Enter)),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert!(screen.rules().is_on(1, Rule::Portamento));
    assert!(!screen.rules().is_on(0, Rule::Portamento));
    // 切り替えても開いたまま。
    assert_eq!(screen.rule_list_selected(), Some(portamento));

    screen.handle_key_event(key(KeyCode::Enter));
    assert!(screen.rules().is_empty());

    screen.handle_key_event(key(KeyCode::Esc));
    assert_eq!(screen.rule_list_selected(), None);
}

#[test]
fn keys_go_to_the_list_while_it_is_open() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(key(KeyCode::Char('t')));
    // `l` は列移動ではなく、`m` もルールの切り替えにならない。
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('m')));
    assert_eq!(screen.cursor(), 0);
    assert!(screen.rules().is_empty());
    // 先頭で k、末尾で j を押しても範囲内に留まる。
    screen.handle_key_event(key(KeyCode::Char('k')));
    assert_eq!(screen.rule_list_selected(), Some(0));
    for _ in 0..RULE_ROWS.len() + 2 {
        screen.handle_key_event(key(KeyCode::Char('j')));
    }
    assert_eq!(screen.rule_list_selected(), Some(RULE_ROWS.len() - 1));
}

#[test]
fn t_without_mml_does_not_open_the_list() {
    let mut screen = GuitarArticulationScreen::default();
    screen.handle_key_event(key(KeyCode::Char('t')));
    assert_eq!(screen.rule_list_selected(), None);
    assert!(screen.error.is_some());
}
