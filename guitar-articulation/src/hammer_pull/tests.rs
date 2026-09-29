use super::*;
use crate::Rule;

/// `(on 秒, pitch)` の列から音符を作る。同じ on 秒は同じ列。長さは 0.5 秒。
fn notes(spec: &[(f64, u8)]) -> Vec<Note> {
    let mut column = 0;
    spec.iter()
        .enumerate()
        .map(|(i, &(on, pitch))| {
            if i > 0 && on != spec[i - 1].0 {
                column += 1;
            }
            Note {
                on_seconds: on,
                off_seconds: on + 0.5,
                channel: 0,
                pitch,
                velocity: 100,
                column,
            }
        })
        .collect()
}

fn rules_on(columns: &[usize]) -> RuleTable {
    let mut rules = RuleTable::default();
    for &c in columns {
        rules.toggle(c, Rule::HammerPull);
    }
    rules
}

use Articulation::{HammerOn, PullOff, SusDown};

#[test]
fn ascending_is_hammer_on_and_descending_is_pull_off() {
    let n = notes(&[(0.0, 40), (0.5, 42), (1.0, 40)]);
    assert_eq!(
        apply_hammer_pull(&n, &rules_on(&[1, 2])),
        vec![SusDown, HammerOn, PullOff]
    );
}

#[test]
fn only_columns_with_the_rule_change() {
    let n = notes(&[(0.0, 40), (0.5, 42), (1.0, 43)]);
    assert_eq!(
        apply_hammer_pull(&n, &rules_on(&[2])),
        vec![SusDown, SusDown, HammerOn]
    );
}

#[test]
fn same_pitch_keeps_the_default() {
    let n = notes(&[(0.0, 40), (0.5, 40)]);
    assert_eq!(
        apply_hammer_pull(&n, &rules_on(&[1])),
        vec![SusDown, SusDown]
    );
}

#[test]
fn first_column_is_not_a_target() {
    let n = notes(&[(0.0, 40), (0.5, 42)]);
    assert_eq!(
        apply_hammer_pull(&n, &rules_on(&[0])),
        vec![SusDown, SusDown]
    );
}

#[test]
fn chord_in_this_column_is_not_a_target() {
    let n = notes(&[(0.0, 40), (0.5, 42), (0.5, 45)]);
    assert_eq!(
        apply_hammer_pull(&n, &rules_on(&[1])),
        vec![SusDown, SusDown, SusDown]
    );
}

#[test]
fn chord_in_the_previous_column_is_not_a_target() {
    let n = notes(&[(0.0, 40), (0.0, 45), (0.5, 47)]);
    assert_eq!(
        apply_hammer_pull(&n, &rules_on(&[1])),
        vec![SusDown, SusDown, SusDown]
    );
}

#[test]
fn toggling_twice_removes_the_rule() {
    let mut rules = rules_on(&[1]);
    rules.toggle(1, Rule::HammerPull);
    assert!(rules.is_empty());
    assert!(!rules.is_on(1, Rule::HammerPull));
}

#[test]
fn auto_picks_the_first_note_of_each_string_and_legatos_the_rest() {
    // 40〜43 と 46〜42 と 40 の 3 本（1 本で押さえる幅は 5 半音まで）。
    let n = notes(&[
        (0.0, 40),
        (0.5, 42),
        (1.0, 43),
        (1.5, 46),
        (2.0, 43),
        (2.5, 42),
        (3.0, 40),
    ]);
    let mut rules = RuleTable::default();
    rules.toggle_row(crate::RowRule::AutoHammerPull);
    assert_eq!(
        apply_hammer_pull(&n, &rules),
        vec![SusDown, HammerOn, HammerOn, SusDown, PullOff, PullOff, SusDown]
    );
}

#[test]
fn a_column_rule_still_applies_on_an_auto_picking_column() {
    let n = notes(&[(0.0, 40), (0.5, 42), (1.0, 43), (1.5, 45)]);
    let mut rules = rules_on(&[3]);
    rules.toggle_row(crate::RowRule::AutoHammerPull);
    assert_eq!(
        apply_hammer_pull(&n, &rules),
        vec![SusDown, HammerOn, HammerOn, HammerOn]
    );
}
