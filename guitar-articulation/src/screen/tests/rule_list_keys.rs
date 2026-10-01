use super::{key, screen_with_mml};
use crate::ui::{RuleLane, RULE_ROWS};
use crate::{GuitarArticulationAction, GuitarArticulationScreen, Rule, Take};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

const PLAY: GuitarArticulationAction = GuitarArticulationAction::Play(Take::Converted);
const CONTINUE: GuitarArticulationAction = GuitarArticulationAction::Continue;

/// 選んでいる段の中で選ばれている項目（`None` は「なし」）。
pub(super) fn choice(screen: &GuitarArticulationScreen) -> Option<Rule> {
    let lane = screen.rule_list_lane().expect("段を選んでいない");
    let listed = screen
        .rule_list_lanes()
        .into_iter()
        .find(|listed| listed.lane == lane)
        .unwrap();
    screen.rule_list_choice(&listed)
}

/// 見えている段と、その中の項目。
pub(super) fn lanes(screen: &GuitarArticulationScreen) -> Vec<(RuleLane, Vec<Rule>)> {
    screen
        .rule_list_lanes()
        .into_iter()
        .map(|listed| {
            let rules = listed.rules.iter().map(|rule_row| rule_row.rule).collect();
            (listed.lane, rules)
        })
        .collect()
}

pub(super) fn press(screen: &mut GuitarArticulationScreen, ch: char) -> GuitarArticulationAction {
    screen.handle_key_event(key(KeyCode::Char(ch)))
}

/// 2 列目にカーソルを置いて overlay を開いた画面。
fn list_on_second_column() -> GuitarArticulationScreen {
    let mut screen = screen_with_mml("o3 l8 e g a");
    press(&mut screen, 'l');
    press(&mut screen, 't');
    screen
}

#[test]
fn opening_selects_the_ks_lane_and_its_choice_follows_the_cursor_column() {
    let mut screen = list_on_second_column();
    assert_eq!(screen.rule_list_lane(), Some(RuleLane::KeySwitch));
    assert_eq!(choice(&screen), None);

    press(&mut screen, 'p');
    assert_eq!(choice(&screen), Some(Rule::PinchHarmonic));
    // 閉じて開き直しても、選ばれているのはカーソル列で ON のもの。
    screen.handle_key_event(key(KeyCode::Esc));
    press(&mut screen, 't');
    assert_eq!(screen.rule_list_lane(), Some(RuleLane::KeySwitch));
    assert_eq!(choice(&screen), Some(Rule::PinchHarmonic));
}

#[test]
fn l_turns_the_next_ks_rule_on_and_h_walks_back_to_none() {
    let mut screen = list_on_second_column();
    assert_eq!(press(&mut screen, 'l'), PLAY);
    assert!(screen.rules().is_on(1, Rule::HammerPull));
    assert!(!screen.rules().is_on(0, Rule::HammerPull));

    assert_eq!(press(&mut screen, 'l'), PLAY);
    assert!(screen.rules().is_on(1, Rule::PalmMute));
    assert!(!screen.rules().is_on(1, Rule::HammerPull));
    assert_eq!(choice(&screen), Some(Rule::PalmMute));

    assert_eq!(press(&mut screen, 'h'), PLAY);
    assert_eq!(press(&mut screen, 'h'), PLAY);
    assert!(screen.rules().is_empty());
    assert_eq!(choice(&screen), None);
    // 左端（なし）で h を押しても変わらず、鳴らさない。
    assert_eq!(press(&mut screen, 'h'), CONTINUE);
    assert!(screen.rules().is_empty());

    // ←→ は h/l と同じ。
    assert_eq!(screen.handle_key_event(key(KeyCode::Right)), PLAY);
    assert!(screen.rules().is_on(1, Rule::HammerPull));
    assert_eq!(screen.handle_key_event(key(KeyCode::Left)), PLAY);
    assert!(screen.rules().is_empty());
}

#[test]
fn the_ks_lane_stops_at_its_right_end_and_runs_through_its_three_rows() {
    let mut screen = list_on_second_column();
    press(&mut screen, 'J');
    assert_eq!(choice(&screen), Some(Rule::EffectHardStop));
    assert_eq!(press(&mut screen, 'l'), CONTINUE);
    assert!(screen.rules().is_on(1, Rule::EffectHardStop));

    // アタックとリリースの行の最後（fret mute）の次は、ピッチの行の先頭（slide up/down）。
    press(&mut screen, 'e');
    assert_eq!(choice(&screen), Some(Rule::FretMute));
    assert_eq!(press(&mut screen, 'l'), PLAY);
    assert!(screen.rules().is_on(1, Rule::Slide));
    assert!(!screen.rules().is_on(1, Rule::FretMute));
}

#[test]
fn j_moves_to_the_vibrato_lane_and_l_stacks_vibrato_on_the_ks_rule() {
    let mut screen = list_on_second_column();
    press(&mut screen, 'l');
    press(&mut screen, 'j');
    assert_eq!(
        screen.rule_list_lane(),
        Some(RuleLane::Single(Rule::Vibrato))
    );
    assert_eq!(choice(&screen), None);
    assert_eq!(press(&mut screen, 'l'), PLAY);
    assert!(screen.rules().is_on(1, Rule::Vibrato));
    assert!(screen.rules().is_on(1, Rule::HammerPull));
    // 1 つしか無い段の右端。
    assert_eq!(press(&mut screen, 'l'), CONTINUE);
}

#[test]
fn the_release_lane_switches_between_position_rel_and_auto_slide_out() {
    let mut screen = list_on_second_column();
    for _ in 0..4 {
        screen.handle_key_event(key(KeyCode::Down));
    }
    assert_eq!(screen.rule_list_lane(), Some(RuleLane::ReleaseShape));
    assert_eq!(press(&mut screen, 'l'), PLAY);
    assert!(screen.rules().is_on(1, Rule::PositionRelease));
    assert_eq!(press(&mut screen, 'l'), PLAY);
    assert!(screen.rules().is_on(1, Rule::AutoSlideOut));
    assert!(!screen.rules().is_on(1, Rule::PositionRelease));

    // 末尾で j、先頭で k を押しても範囲内に留まる。
    press(&mut screen, 'j');
    assert_eq!(screen.rule_list_lane(), Some(RuleLane::ReleaseShape));
    for _ in 0..6 {
        screen.handle_key_event(key(KeyCode::Up));
    }
    assert_eq!(screen.rule_list_lane(), Some(RuleLane::KeySwitch));
}

#[test]
fn an_overlay_letter_toggles_its_rule_and_selects_its_lane() {
    let mut screen = list_on_second_column();
    assert_eq!(press(&mut screen, 'v'), PLAY);
    assert!(screen.rules().is_on(1, Rule::Vibrato));
    assert_eq!(
        screen.rule_list_lane(),
        Some(RuleLane::Single(Rule::Vibrato))
    );
    press(&mut screen, 'v');
    assert!(screen.rules().is_empty());

    for rule_row in RULE_ROWS {
        let letter = rule_row.overlay_key;
        assert_eq!(press(&mut screen, letter), PLAY, "{letter}");
        assert!(screen.rules().is_on(1, rule_row.rule), "{letter}");
        assert_eq!(
            screen.rule_list_lane(),
            Some(RuleLane::of(rule_row.rule)),
            "{letter}"
        );
        press(&mut screen, letter);
        assert!(screen.rules().is_empty(), "{letter}");
    }
    // 大文字は Shift 付きで届いても同じルール。
    let unison_bend = RULE_ROWS
        .iter()
        .find(|rule_row| rule_row.overlay_key == 'A')
        .unwrap()
        .rule;
    screen.handle_key_event(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT));
    assert!(screen.rules().is_on(1, unison_bend));
}

#[test]
fn space_previews_without_changing_the_rules() {
    let mut screen = list_on_second_column();
    assert_eq!(press(&mut screen, ' '), PLAY);
    assert!(screen.rules().is_empty());
    assert_eq!(screen.rule_list_lane(), Some(RuleLane::KeySwitch));
}

#[test]
fn space_plays_only_the_cursor_column_in_the_note_preview() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    press(&mut screen, 'n');
    press(&mut screen, 'l');
    press(&mut screen, 't');
    assert_eq!(
        press(&mut screen, ' '),
        GuitarArticulationAction::PlayNote {
            take: Take::Converted,
            column: 1,
        }
    );
}

#[test]
fn enter_and_esc_close_the_list_without_toggling() {
    for code in [KeyCode::Enter, KeyCode::Esc] {
        let mut screen = list_on_second_column();
        assert_eq!(screen.handle_key_event(key(code)), CONTINUE);
        assert!(!screen.rule_list_open(), "{code:?}");
        assert_eq!(screen.rule_list_lane(), None, "{code:?}");
        assert!(screen.rules().is_empty(), "{code:?}");
    }
}

#[test]
fn h_and_l_do_not_move_the_cursor_while_the_list_is_open() {
    let mut screen = screen_with_mml("o3 l8 e g a");
    press(&mut screen, 't');
    press(&mut screen, 'l');
    press(&mut screen, 'l');
    assert_eq!(screen.cursor(), 0);
    assert!(screen.rules().is_on(0, Rule::PalmMute));
}

#[test]
fn t_without_mml_does_not_open_the_list() {
    let mut screen = GuitarArticulationScreen::default();
    press(&mut screen, 't');
    assert!(!screen.rule_list_open());
    assert!(screen.error.is_some());
}
