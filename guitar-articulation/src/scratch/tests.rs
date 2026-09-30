use super::*;
use crate::{articulate, column_events, convert, notes_from_events, RowRule, Take};

fn raw(mml: &str) -> Vec<TimedMidiEvent> {
    cmrt_chord::timed_performance(mml).unwrap().events
}

fn scratch_columns(columns: &[usize]) -> RuleTable {
    let mut rules = RuleTable::default();
    for &column in columns {
        rules.toggle(column, Rule::PickScratch);
    }
    rules
}

/// 演奏音の note on の音高（KS を除く）。
fn played_pitches(events: &[TimedMidiEvent]) -> Vec<u8> {
    events
        .iter()
        .filter(|e| is_note_on(&e.message) && Articulation::from_keyswitch(e.message[1]).is_none())
        .map(|e| e.message[1])
        .collect()
}

#[test]
fn pitches_fold_into_the_sample_range_by_octaves() {
    for (pitch, folded) in [
        (30, 30),
        (42, 42),
        (43, 31),
        (29, 41),
        (60, 36),
        (0, 36),
        (127, 31),
    ] {
        assert_eq!(pick_scratch_pitch(pitch), folded, "{pitch}");
        assert!(PICK_SCRATCH_PITCHES.contains(&folded));
    }
}

#[test]
fn scratch_overrides_the_row_rules_in_its_column() {
    let mut rules = scratch_columns(&[1]);
    rules.toggle_row(RowRule::AutoHammerPull);
    let articulations: Vec<Articulation> =
        articulate(&notes_from_events(&raw("o3 l8 e f+ g")), &rules)
            .iter()
            .map(|a| a.articulation)
            .collect();
    assert_eq!(
        articulations,
        vec![
            Articulation::SusDown,
            Articulation::PickScratch,
            Articulation::HammerOn,
        ]
    );
}

#[test]
fn scratch_is_exclusive_with_the_other_voicing_rules_but_stacks_with_vibrato() {
    let mut rules = RuleTable::default();
    rules.toggle(0, Rule::PalmMute);
    rules.toggle(0, Rule::Vibrato);
    rules.toggle(0, Rule::PickScratch);
    assert!(!rules.is_on(0, Rule::PalmMute));
    assert!(rules.is_on(0, Rule::Vibrato));
    assert!(rules.is_on(0, Rule::PickScratch));
}

#[test]
fn scratch_round_trips_through_json() {
    let rules = scratch_columns(&[2]);
    assert_eq!(
        rules.to_json(),
        r#"{"columns":{"2":["pick_scratch"]},"rows":[]}"#
    );
    assert_eq!(RuleTable::from_json(&rules.to_json()).unwrap(), rules);
}

#[test]
fn converted_scratch_note_sounds_folded_after_its_keyswitch() {
    // E4 = 64 は 40 へ畳む。
    let out = convert(&raw("o5 l8 c e g"), &scratch_columns(&[1]));
    assert_eq!(played_pitches(&out), vec![60, 40, 67]);
    assert!(
        !out.iter().any(|e| e.message[1] == 64),
        "元の音高の note on/off が残っている"
    );
    let on = |key: u8| {
        out.iter()
            .position(|e| is_note_on(&e.message) && e.message[1] == key)
            .unwrap()
    };
    assert!(on(Articulation::PickScratch.keyswitch()) < on(40));
    let off_40 = out
        .iter()
        .find(|e| is_note_off(&e.message) && e.message[1] == 40)
        .unwrap();
    assert!((off_40.seconds - 0.5).abs() < 1e-9, "{off_40:?}");
}

#[test]
fn a_scratch_chord_sounds_only_its_lowest_note() {
    let out = convert(&raw("o5 l4 'ceg' c"), &scratch_columns(&[0]));
    assert_eq!(played_pitches(&out), vec![36, 60]);
    let offs = out
        .iter()
        .filter(|e| is_note_off(&e.message) && [36, 64, 67].contains(&e.message[1]))
        .count();
    assert_eq!(offs, 1);
}

#[test]
fn humanized_scratch_sounds_folded_too() {
    let mut rules = scratch_columns(&[0, 1]);
    rules.toggle_row(RowRule::Humanize);
    let out = convert(&raw("o5 l8 'ce' g"), &rules);
    assert_eq!(played_pitches(&out), vec![36, 31]);
}

#[test]
fn single_note_preview_folds_only_the_converted_take() {
    let events = raw("o5 l8 'ce' g");
    let notes = notes_from_events(&events);
    let rules = scratch_columns(&[0]);
    let articulated = articulate(&notes, &rules);
    let pitches = |take| played_pitches(&column_events(&notes, &articulated, &rules, 0, take));
    assert_eq!(pitches(Take::Converted), vec![36]);
    assert_eq!(pitches(Take::Plain), vec![60, 64]);
}
