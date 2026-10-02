use super::{key, screen_with_mml};
use crate::history::GuitarArticulationHistoryEntry;
use crate::{convert, ColumnRuleAnchor, GuitarArticulationScreen, Rule, RuleTable, Take};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// `i` → 今の MML を消す → 文字 → `Enter` で MML を確定し直す。
fn recommit(screen: &mut GuitarArticulationScreen, mml: &str) {
    screen.handle_key_event(key(KeyCode::Char('i')));
    for _ in 0..screen.mml().chars().count() {
        screen.handle_key_event(key(KeyCode::Backspace));
    }
    for ch in mml.chars() {
        screen.handle_key_event(key(KeyCode::Char(ch)));
    }
    screen.handle_key_event(key(KeyCode::Enter));
    assert_eq!(screen.mml(), mml);
}

/// 列ごとのルールが ON の (列, ルール) を全部。
fn column_rules(screen: &GuitarArticulationScreen) -> Vec<(usize, Rule)> {
    let mut on = Vec::new();
    for column in 0..screen.column_count() {
        for rule in crate::ui::RULE_ROWS.map(|rule_row| rule_row.rule) {
            if screen.rules().is_on(column, rule) {
                on.push((column, rule));
            }
        }
    }
    on
}

/// 画面の Articulated が、今のルール表で作り直したものと同じか。
fn assert_converted_matches_rules(screen: &GuitarArticulationScreen) {
    let plain = screen.events(Take::Plain).to_vec();
    assert_eq!(
        screen.events(Take::Converted),
        convert(&plain, screen.rules()).as_slice()
    );
}

#[test]
fn a_column_rule_stays_at_its_time_while_the_pitches_change() {
    let mut screen = screen_with_mml("c e g");
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('g')));

    for mml in ["c g g", "c c g", "a 'ceg' b", "c e g"] {
        recommit(&mut screen, mml);
        assert_eq!(column_rules(&screen), vec![(1, Rule::PickScratch)], "{mml}");
        assert_converted_matches_rules(&screen);
    }
}

#[test]
fn a_column_rule_stays_at_its_time_when_a_note_is_inserted_in_front() {
    let mut screen = screen_with_mml("c e g");
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('g')));

    recommit(&mut screen, "c c e g");

    assert_eq!(column_rules(&screen), vec![(1, Rule::PickScratch)]);
}

#[test]
fn a_rule_past_the_end_moves_to_the_last_column_and_comes_back() {
    let mut screen = screen_with_mml("c e g");
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('g')));

    recommit(&mut screen, "c e");
    assert_eq!(column_rules(&screen), vec![(1, Rule::PickScratch)]);

    recommit(&mut screen, "c e g");
    assert_eq!(column_rules(&screen), vec![(2, Rule::PickScratch)]);
}

#[test]
fn only_toggling_a_column_rule_updates_the_anchor() {
    let mut screen = screen_with_mml("c e g");
    assert_eq!(screen.anchor, None);
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('g')));
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::PickScratch);
    let anchor = Some(ColumnRuleAnchor::new("c e g", &rules));
    assert_eq!(screen.anchor, anchor);

    screen.handle_key_event(key(KeyCode::Char('e')));
    recommit(&mut screen, "c c e g");
    screen.handle_key_event(key(KeyCode::Char(' ')));

    assert_eq!(screen.anchor, anchor);
}

#[test]
fn an_entry_without_an_anchor_uses_its_own_mml_and_column_rules_as_the_anchor() {
    let entry: GuitarArticulationHistoryEntry = serde_json::from_str(
        r#"{"mml":"c e g","rules":{"columns":{"1":["pick_scratch"]},"rows":[]},"effect_chain":[]}"#,
    )
    .unwrap();
    assert_eq!(entry.anchor, None);
    let mut screen = screen_with_mml("a");
    screen.apply_history_entry(&entry);

    recommit(&mut screen, "c c e g");

    assert_eq!(column_rules(&screen), vec![(1, Rule::PickScratch)]);
}

#[test]
fn esc_on_the_history_overlay_restores_the_anchor_too() {
    let mut screen = screen_with_mml("c e g");
    screen.handle_key_event(key(KeyCode::Char('g')));
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::Char('m')));
    let before = screen.anchor.clone();
    let shift_h = KeyEvent::new(KeyCode::Char('H'), KeyModifiers::SHIFT);

    screen.handle_key_event(shift_h);
    screen.handle_key_event(key(KeyCode::Char('j')));
    assert_ne!(screen.anchor, before);
    screen.handle_key_event(key(KeyCode::Esc));

    assert_eq!(screen.anchor, before);
}
