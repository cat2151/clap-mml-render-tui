use super::{key, screen_with_mml};
use crate::{GuitarArticulationAction, Take, PARAMS};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

const MUTE_LENGTH: usize = 1;

#[test]
fn u_then_j_then_h_l_changes_the_selected_param_and_plays() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(key(KeyCode::Char('u')));
    assert_eq!(screen.param_list_selected(), Some(0));
    screen.handle_key_event(key(KeyCode::Char('j')));
    assert_eq!(screen.param_list_selected(), Some(MUTE_LENGTH));
    let cc = PARAMS[MUTE_LENGTH].cc;

    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('h'))),
        GuitarArticulationAction::Play(Take::Converted)
    );
    assert_eq!(screen.rules().param(cc), 43);
    assert!(screen
        .events(Take::Converted)
        .iter()
        .any(|e| e.message == [0xB0, cc, 43]));
    // `h` / `l` は列移動にならない。
    assert_eq!(screen.cursor(), 0);
    screen.handle_key_event(key(KeyCode::Right));
    assert_eq!(screen.rules().param(cc), 51);
    assert!(screen.rules().changed_params().next().is_none());

    screen.handle_key_event(key(KeyCode::Esc));
    assert_eq!(screen.param_list_selected(), None);
}

#[test]
fn keys_go_to_the_param_list_while_it_is_open() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(key(KeyCode::Char('u')));
    screen.handle_key_event(key(KeyCode::Char('m')));
    assert!(screen.rules().is_empty());
    screen.handle_key_event(key(KeyCode::Char('k')));
    assert_eq!(screen.param_list_selected(), Some(0));
    for _ in 0..PARAMS.len() + 2 {
        screen.handle_key_event(key(KeyCode::Char('j')));
    }
    assert_eq!(screen.param_list_selected(), Some(PARAMS.len() - 1));
    // CC112 の既定は 0。下の端では変わらず、鳴らさない。
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('h'))),
        GuitarArticulationAction::Continue
    );
}

#[test]
fn params_are_recorded_in_the_history_and_restored_from_it() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    let cc = PARAMS[MUTE_LENGTH].cc;
    screen.handle_key_event(key(KeyCode::Char('u')));
    screen.handle_key_event(key(KeyCode::Char('j')));
    screen.handle_key_event(key(KeyCode::Char('h')));
    assert_eq!(screen.history().entries[0].rules.param(cc), 43);
    screen.handle_key_event(key(KeyCode::Char('h')));
    assert_eq!(screen.history().entries[0].rules.param(cc), 35);
    screen.handle_key_event(key(KeyCode::Esc));

    // 1 つ前の履歴（43）を選んで確定する。
    screen.handle_key_event(KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT));
    screen.handle_key_event(key(KeyCode::Char('j')));
    screen.handle_key_event(key(KeyCode::Enter));
    assert_eq!(screen.rules().param(cc), 43);
    assert!(screen
        .events(Take::Converted)
        .iter()
        .any(|e| e.message == [0xB0, cc, 43]));
}
