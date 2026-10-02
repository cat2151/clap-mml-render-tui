use super::*;
use crate::{RowRule, Rule};

fn notes(mml: &str) -> Vec<Note> {
    notes_from_events(&cmrt_chord::timed_performance(mml).unwrap().events)
}

#[test]
fn new_keeps_only_the_column_rules() {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::PickScratch);
    rules.toggle_row(RowRule::EconomyPicking);

    let anchor = ColumnRuleAnchor::new("c e g", &Default::default(), &rules);

    assert!(anchor.rules.is_on(1, Rule::PickScratch));
    assert!(!anchor.rules.is_row_on(RowRule::EconomyPicking));
}

#[test]
fn rules_for_keeps_the_column_rules_at_their_time_and_takes_the_row_rules_from_current() {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::PickScratch);
    let anchor = ColumnRuleAnchor::new("c e g", &Default::default(), &rules);
    let mut current = RuleTable::default();
    current.toggle(0, Rule::PalmMute);
    current.toggle_row(RowRule::Humanize);

    let result = anchor.rules_for(&notes("d c c g"), &current);

    let mut expected = RuleTable::default();
    expected.toggle(1, Rule::PickScratch);
    expected.toggle_row(RowRule::Humanize);
    assert_eq!(result, expected);
}

#[test]
fn an_unparsable_anchor_gives_no_column_rules() {
    let mut rules = RuleTable::default();
    rules.toggle(0, Rule::PickScratch);
    let anchor = ColumnRuleAnchor::new("[[[", &Default::default(), &rules);
    let mut current = RuleTable::default();
    current.toggle_row(RowRule::Humanize);

    let result = anchor.rules_for(&notes("c e g"), &current);

    assert!(result.is_empty());
    assert!(result.is_row_on(RowRule::Humanize));
}
