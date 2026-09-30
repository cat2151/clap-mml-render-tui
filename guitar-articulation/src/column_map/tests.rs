use super::*;
use crate::{notes_from_events, RowRule, Rule};

fn notes(mml: &str) -> Vec<Note> {
    notes_from_events(&cmrt_chord::timed_performance(mml).unwrap().events)
}

fn map(old: &str, new: &str) -> Vec<Option<usize>> {
    column_map(&notes(old), &notes(new))
}

#[test]
fn same_mml_maps_to_itself() {
    assert_eq!(
        map("o4 l8 c e g", "o4 l8 c e g"),
        vec![Some(0), Some(1), Some(2)]
    );
}

#[test]
fn a_pitch_change_keeps_the_positions() {
    assert_eq!(
        map("o4 l8 c e g", "o4 l8 c g g"),
        vec![Some(0), Some(1), Some(2)]
    );
    assert_eq!(
        map("o4 l8 c g c g", "o4 l8 c c c g"),
        vec![Some(0), Some(1), Some(2), Some(3)]
    );
}

#[test]
fn a_wholly_different_mml_with_the_same_rhythm_keeps_the_positions() {
    let old = "o4 l8 c 'ceg' g";
    assert_eq!(notes(old).len(), 5);
    assert_eq!(
        map(old, "o5 l8 'dfa' b- e"),
        vec![Some(0), Some(1), Some(2)]
    );
}

#[test]
fn a_column_inserted_in_front_keeps_the_positions() {
    assert_eq!(
        map("o4 l8 c e g", "o4 l8 d c e g"),
        vec![Some(0), Some(1), Some(2)]
    );
}

#[test]
fn a_different_rhythm_maps_to_the_nearest_onset() {
    // 旧の note on は 0, 0.25, 0.5 秒、新は 0, 0.75 秒。
    assert_eq!(
        map("o4 l8 c e g", "o4 l4. c e"),
        vec![Some(0), Some(0), Some(1)]
    );
}

#[test]
fn a_tie_goes_to_the_earlier_column() {
    // 旧の 0.25 秒は、新の 0 秒と 0.5 秒から等距離。
    assert_eq!(map("o4 l8 c e", "o4 l4 c e"), vec![Some(0), Some(0)]);
}

#[test]
fn empty_old_or_new() {
    let ceg = notes("o4 l8 c e g");
    assert_eq!(column_map(&[], &ceg), Vec::<Option<usize>>::new());
    assert_eq!(column_map(&ceg, &[]), vec![None, None, None]);
}

#[test]
fn remap_columns_moves_column_rules_and_drops_unmapped_and_row_rules() {
    let mut rules = RuleTable::default();
    rules.toggle(0, Rule::PalmMute);
    rules.toggle(1, Rule::PickScratch);
    rules.toggle(1, Rule::Vibrato);
    rules.toggle(2, Rule::Slide);
    rules.toggle(5, Rule::Choke);
    rules.toggle_row(RowRule::EconomyPicking);

    let remapped = rules.remap_columns(&[Some(1), None, Some(3)]);

    let mut expected = RuleTable::default();
    expected.toggle(1, Rule::PalmMute);
    expected.toggle(3, Rule::Slide);
    assert_eq!(remapped, expected);
    assert!(!remapped.is_row_on(RowRule::EconomyPicking));
}

#[test]
fn remap_columns_merges_columns_that_move_to_the_same_column() {
    let mut rules = RuleTable::default();
    rules.toggle(0, Rule::PickScratch);
    rules.toggle(1, Rule::PalmMute);
    rules.toggle(1, Rule::Vibrato);
    assert!(Rule::PickScratch.is_exclusive() && Rule::PalmMute.is_exclusive());
    assert!(!Rule::Vibrato.is_exclusive());

    let remapped = rules.remap_columns(&[Some(0), Some(0)]);

    // 排他どうしは前の列の PickScratch を残す。
    let mut expected = RuleTable::default();
    expected.toggle(0, Rule::PickScratch);
    expected.toggle(0, Rule::Vibrato);
    assert_eq!(remapped, expected);
}
