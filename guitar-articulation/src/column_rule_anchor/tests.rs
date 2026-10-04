use super::*;
use crate::{RowRule, Rule, VibratoSettings};

fn notes(mml: &str) -> Vec<Note> {
    notes_from_events(&cmrt_chord::timed_performance(mml).unwrap().events)
}

#[test]
fn new_keeps_only_the_column_rules() {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::PickScratch);
    rules.toggle_row(RowRule::EconomyPicking);
    rules.set_vibrato_settings(VibratoSettings {
        delay_ms: 500,
        ..VibratoSettings::default()
    });
    rules.step_param(21, 8);

    let anchor = ColumnRuleAnchor::new("c e g", &rules);

    assert!(anchor.rules.is_on(1, Rule::PickScratch));
    assert!(!anchor.rules.is_row_on(RowRule::EconomyPicking));
    assert_eq!(anchor.rules.vibrato_settings(), VibratoSettings::default());
    assert_eq!(anchor.rules.param(21), 95);
}

#[test]
fn rules_for_keeps_the_column_rules_at_their_time_and_takes_the_row_rules_from_current() {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::PickScratch);
    let anchor = ColumnRuleAnchor::new("c e g", &rules);
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
    let anchor = ColumnRuleAnchor::new("[[[", &rules);
    let mut current = RuleTable::default();
    current.toggle_row(RowRule::Humanize);
    current.set_vibrato_settings(VibratoSettings {
        rise_ms: 650,
        ..VibratoSettings::default()
    });

    let result = anchor.rules_for(&notes("c e g"), &current);

    assert!(result.is_empty());
    assert!(result.is_row_on(RowRule::Humanize));
    assert_eq!(result.vibrato_settings(), current.vibrato_settings());
}

#[test]
fn remapping_uses_current_phrase_settings_instead_of_saved_anchor_settings() {
    let anchor: ColumnRuleAnchor = serde_json::from_str(
        r#"{"mml":"c e g","rules":{"columns":{"1":["vibrato"]},"vibrato":{"delay_ms":100,"rise_ms":200,"depth":32},"params":{"21":87}}}"#,
    )
    .unwrap();
    let mut current = RuleTable::default();
    current.set_vibrato_settings(VibratoSettings {
        delay_ms: 1000,
        rise_ms: 1500,
        depth: 112,
    });
    current.step_param(21, 8);

    let result = anchor.rules_for(&notes("d c c g"), &current);

    assert!(result.is_on(1, Rule::Vibrato));
    assert_eq!(result.vibrato_settings(), current.vibrato_settings());
    assert_eq!(result.param(21), 103);
}

#[test]
fn an_anchor_saved_with_an_arp_field_reads_without_it() {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::PickScratch);
    let saved: ColumnRuleAnchor = serde_json::from_str(
        r#"{"mml":"c e g","arp":{"pattern":"UpDown"},"rules":{"columns":{"1":["pick_scratch"]},"rows":[]}}"#,
    )
    .unwrap();

    assert_eq!(saved, ColumnRuleAnchor::new("c e g", &rules));
}
