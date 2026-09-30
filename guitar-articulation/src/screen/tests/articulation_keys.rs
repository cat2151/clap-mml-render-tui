use super::{key, screen_with_mml};
use crate::{GuitarArticulationAction, Rule, Take};
use crossterm::event::KeyCode;

#[test]
fn m_p_slash_c_v_toggle_their_rule_on_the_cursor_column_and_play_the_new_take() {
    for (ch, rule) in [
        ('m', Rule::PalmMute),
        ('p', Rule::PinchHarmonic),
        ('/', Rule::Slide),
        ('c', Rule::Choke),
        ('v', Rule::Vibrato),
    ] {
        let mut screen = screen_with_mml("o3 l8 e g a");
        screen.handle_key_event(key(KeyCode::Char('l')));

        assert_eq!(
            screen.handle_key_event(key(KeyCode::Char(ch))),
            GuitarArticulationAction::Play(Take::Converted),
            "{ch}"
        );
        assert!(screen.rules().is_on(1, rule), "{ch}");
        assert!(!screen.rules().is_on(0, rule), "{ch}");

        screen.handle_key_event(key(KeyCode::Char(ch)));
        assert!(screen.rules().is_empty(), "{ch}");
    }
}

#[test]
fn a_voicing_rule_replaces_hammer_pull_but_vibrato_stacks() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    screen.handle_key_event(key(KeyCode::Char('l')));
    for ch in ['a', 'v', 'm'] {
        screen.handle_key_event(key(KeyCode::Char(ch)));
    }

    assert!(!screen.rules().is_on(1, Rule::HammerPull));
    assert!(screen.rules().is_on(1, Rule::PalmMute));
    assert!(screen.rules().is_on(1, Rule::Vibrato));
}
