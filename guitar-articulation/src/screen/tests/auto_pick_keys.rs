use super::{key, screen_with_mml};
use crate::{Articulation, AutoPick, RowRule, RuleTable};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use Articulation::{HammerOn, PullOff, SusDown, SusUp};

fn articulations(screen: &crate::GuitarArticulationScreen) -> Vec<Articulation> {
    screen
        .articulated()
        .iter()
        .map(|a| a.articulation)
        .collect()
}

fn on2(mml: &str) -> crate::GuitarArticulationScreen {
    let mut screen = screen_with_mml(mml);
    screen.handle_key_event(key(KeyCode::Char('s')));
    screen.handle_key_event(key(KeyCode::Char('s')));
    assert_eq!(screen.rules().auto_pick(), AutoPick::Accent);
    screen
}

#[test]
fn on2_picks_only_the_head_and_the_accented_turns() {
    // c e d f c: 頂点は e と f、谷は d。
    let mut screen = on2("l16cedfc");
    assert_eq!(
        articulations(&screen),
        vec![SusDown, SusUp, PullOff, SusDown, PullOff]
    );

    screen.handle_key_event(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT));
    assert_eq!(
        articulations(&screen),
        vec![SusDown, HammerOn, SusUp, HammerOn, PullOff]
    );
}

#[test]
fn on1_keeps_picking_every_turn() {
    let mut screen = screen_with_mml("l16cedfc");
    screen.handle_key_event(key(KeyCode::Char('s')));
    assert_eq!(screen.rules().auto_pick(), AutoPick::Run);
    assert_ne!(articulations(&screen), articulations(&on2("l16cedfc")));
}

#[test]
fn economy_picking_turns_on2_off_and_back_to_on1_next_time() {
    let mut screen = on2("l16cedfc");
    screen.handle_key_event(key(KeyCode::Char('e')));
    assert!(!screen.rules().is_row_on(RowRule::AutoHammerPull));
    assert_eq!(screen.rules().auto_pick(), AutoPick::Run);

    screen.handle_key_event(key(KeyCode::Char('s')));
    assert_eq!(screen.rules().auto_pick(), AutoPick::Run);
}

#[test]
fn on2_survives_a_new_mml_and_the_json() {
    let mut screen = on2("l16cedfc");
    screen.handle_key_event(key(KeyCode::Char('i')));
    screen.handle_key_event(key(KeyCode::Enter));
    assert_eq!(screen.rules().auto_pick(), AutoPick::Accent);

    let json = screen.rules().to_json();
    assert!(json.contains(r#""auto_pick":"accent""#), "{json}");
    assert_eq!(
        RuleTable::from_json(&json).unwrap().auto_pick(),
        AutoPick::Accent
    );
    // 自動ハンマリングが OFF の JSON に残っていても on2 は持たない。
    let off = RuleTable::from_json(r#"{"auto_pick":"accent"}"#).unwrap();
    assert_eq!(off.auto_pick(), AutoPick::Run);
}
