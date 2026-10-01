use super::rule_list_keys::{choice, lanes, press};
use super::{key, screen_with_mml};
use crate::ui::{RuleLane, RULE_LANES};
use crate::{GuitarArticulationAction, GuitarArticulationScreen, Rule, Take};
use crossterm::event::KeyCode;

const PLAY: GuitarArticulationAction = GuitarArticulationAction::Play(Take::Converted);
const CONTINUE: GuitarArticulationAction = GuitarArticulationAction::Continue;
const TRILLS: [Rule; 4] = [
    Rule::TrillHalf,
    Rule::TrillWhole,
    Rule::TrillMinorThird,
    Rule::TrillMajorThird,
];

fn type_text(screen: &mut GuitarArticulationScreen, text: &str) {
    for ch in text.chars() {
        press(screen, ch);
    }
}

/// 2 列目にカーソルを置いて overlay を開き、`/` で `query` を打った画面（入力中のまま）。
fn filtering(query: &str) -> GuitarArticulationScreen {
    let mut screen = screen_with_mml("o3 l8 e g a");
    press(&mut screen, 'l');
    press(&mut screen, 't');
    press(&mut screen, '/');
    assert!(screen.rule_list_filter_input_active());
    type_text(&mut screen, query);
    screen
}

#[test]
fn scrape_finds_pick_scratch_by_its_alias() {
    let screen = filtering("scrape");
    assert_eq!(
        lanes(&screen),
        vec![(RuleLane::KeySwitch, vec![Rule::PickScratch])]
    );
    assert_eq!(screen.rule_list_lane(), Some(RuleLane::KeySwitch));
}

#[test]
fn scratch_finds_pick_scratch_by_its_name() {
    let screen = filtering("scratch");
    assert_eq!(
        lanes(&screen),
        vec![(RuleLane::KeySwitch, vec![Rule::PickScratch])]
    );
}

#[test]
fn slide_out_finds_the_manual_and_the_auto_slide_out_in_their_own_lanes() {
    let screen = filtering("slide out");
    assert_eq!(
        lanes(&screen),
        vec![
            (RuleLane::KeySwitch, vec![Rule::SlideOut]),
            (RuleLane::ReleaseShape, vec![Rule::AutoSlideOut]),
        ]
    );
}

#[test]
fn h_and_l_walk_only_the_filtered_items() {
    let mut screen = filtering("trill");
    assert_eq!(lanes(&screen), vec![(RuleLane::KeySwitch, TRILLS.to_vec())]);
    screen.handle_key_event(key(KeyCode::Enter));
    assert!(!screen.rule_list_filter_input_active());
    assert!(screen.rule_list_open());
    assert_eq!(screen.rule_list_query(), "trill");

    for &trill in &TRILLS {
        assert_eq!(press(&mut screen, 'l'), PLAY);
        assert!(screen.rules().is_on(1, trill), "{trill:?}");
        assert_eq!(choice(&screen), Some(trill));
    }
    assert_eq!(press(&mut screen, 'l'), CONTINUE);
    for _ in 0..TRILLS.len() {
        assert_eq!(press(&mut screen, 'h'), PLAY);
    }
    assert!(screen.rules().is_empty());
    assert_eq!(press(&mut screen, 'h'), CONTINUE);
    // 見えている段は 1 つだけなので j/k でも動かない。
    press(&mut screen, 'j');
    assert_eq!(screen.rule_list_lane(), Some(RuleLane::KeySwitch));
}

#[test]
fn a_hidden_lane_hands_the_selection_to_the_first_visible_lane() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    press(&mut screen, 't');
    press(&mut screen, 'j');
    press(&mut screen, '/');
    type_text(&mut screen, "vib");
    assert_eq!(
        screen.rule_list_lane(),
        Some(RuleLane::Single(Rule::Vibrato))
    );
    for _ in 0.."vib".len() {
        screen.handle_key_event(key(KeyCode::Backspace));
    }
    type_text(&mut screen, "trill");
    assert_eq!(screen.rule_list_lane(), Some(RuleLane::KeySwitch));
}

#[test]
fn a_letter_for_a_hidden_lane_toggles_its_rule_but_keeps_the_selected_lane() {
    let mut screen = filtering("trill");
    screen.handle_key_event(key(KeyCode::Enter));
    assert_eq!(press(&mut screen, 'v'), PLAY);
    assert!(screen.rules().is_on(1, Rule::Vibrato));
    assert_eq!(screen.rule_list_lane(), Some(RuleLane::KeySwitch));
}

#[test]
fn esc_restores_the_query_and_the_lane_from_before_the_slash() {
    let mut screen = filtering("trill");
    screen.handle_key_event(key(KeyCode::Enter));
    let lanes_before = lanes(&screen);

    press(&mut screen, '/');
    // 入力欄は確定済みの条件から始まる。
    type_text(&mut screen, "x");
    assert_eq!(screen.rule_list_query(), "trillx");
    for _ in 0.."trillx".len() {
        screen.handle_key_event(key(KeyCode::Backspace));
    }
    type_text(&mut screen, "vib");
    assert_eq!(
        lanes(&screen),
        vec![(RuleLane::Single(Rule::Vibrato), vec![Rule::Vibrato])]
    );
    assert_eq!(
        screen.rule_list_lane(),
        Some(RuleLane::Single(Rule::Vibrato))
    );
    screen.handle_key_event(key(KeyCode::Esc));

    assert!(!screen.rule_list_filter_input_active());
    assert!(screen.rule_list_open(), "Esc は入力だけを閉じる");
    assert_eq!(screen.rule_list_query(), "trill");
    assert_eq!(lanes(&screen), lanes_before);
    assert_eq!(screen.rule_list_lane(), Some(RuleLane::KeySwitch));
}

#[test]
fn clearing_the_query_and_enter_releases_the_filter() {
    let mut screen = filtering("trill");
    screen.handle_key_event(key(KeyCode::Enter));
    press(&mut screen, '/');
    for _ in 0.."trill".len() {
        screen.handle_key_event(key(KeyCode::Backspace));
    }
    screen.handle_key_event(key(KeyCode::Enter));
    assert_eq!(screen.rule_list_query(), "");
    assert_eq!(lanes(&screen).len(), RULE_LANES.len());
    assert_eq!(screen.rule_list_lane(), Some(RuleLane::KeySwitch));
}

#[test]
fn letters_go_to_the_input_while_typing() {
    // a / v / l / h / space / j / k はどれも、入力欄の外ではルールか段を動かすキー。
    let screen = filtering("avlh jk");
    assert!(screen.rules().is_empty());
    assert_eq!(screen.rule_list_query(), "avlh jk");
    assert!(screen.rule_list_filter_input_active());
}

#[test]
fn no_match_hides_every_lane_and_lane_keys_do_nothing() {
    let mut screen = filtering("zzz");
    assert!(lanes(&screen).is_empty());
    assert_eq!(screen.rule_list_lane(), None);
    screen.handle_key_event(key(KeyCode::Enter));
    for ch in ['l', 'h', 'j', 'k'] {
        assert_eq!(press(&mut screen, ch), CONTINUE, "{ch}");
    }
    assert!(screen.rules().is_empty());
    assert_eq!(press(&mut screen, ' '), PLAY);
    // 条件が戻れば先頭の段を選ぶ。
    press(&mut screen, '/');
    for _ in 0..3 {
        screen.handle_key_event(key(KeyCode::Backspace));
    }
    assert_eq!(screen.rule_list_lane(), Some(RuleLane::KeySwitch));
    screen.handle_key_event(key(KeyCode::Enter));
    screen.handle_key_event(key(KeyCode::Enter));
    assert!(!screen.rule_list_open());
}

#[test]
fn an_invalid_regex_keeps_the_previous_condition() {
    let mut screen = filtering("trill");
    let before = lanes(&screen);
    type_text(&mut screen, " (");
    assert_eq!(screen.rule_list_query(), "trill (");
    assert_eq!(lanes(&screen), before);
    type_text(&mut screen, "min)");
    assert_eq!(
        lanes(&screen),
        vec![(RuleLane::KeySwitch, vec![Rule::TrillMinorThird])]
    );
}

#[test]
fn reopening_the_list_starts_with_an_empty_query() {
    let mut screen = filtering("trill");
    screen.handle_key_event(key(KeyCode::Enter));
    screen.handle_key_event(key(KeyCode::Esc));
    assert!(!screen.rule_list_open());
    press(&mut screen, 't');
    assert_eq!(screen.rule_list_query(), "");
    assert_eq!(lanes(&screen).len(), RULE_LANES.len());
    assert_eq!(screen.rule_list_lane(), Some(RuleLane::KeySwitch));
}

#[test]
fn the_textarea_cursor_shows_only_while_typing() {
    let mut screen = filtering("tr");
    assert!(screen.uses_textarea_cursor());
    screen.handle_key_event(key(KeyCode::Enter));
    assert!(!screen.uses_textarea_cursor());
}
