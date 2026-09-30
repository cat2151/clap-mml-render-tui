use super::*;
use crate::{articulate, notes_from_events, RowRule};

use Articulation::{HammerOn, MuteDown, MuteUp, PinchHarmonic, SusDown};

fn notes(mml: &str) -> Vec<Note> {
    notes_from_events(&cmrt_chord::timed_performance(mml).unwrap().events)
}

fn articulations(mml: &str, rules: &RuleTable) -> Vec<Articulation> {
    articulate(&notes(mml), rules)
        .iter()
        .map(|a| a.articulation)
        .collect()
}

fn columns(rule: Rule, columns: &[usize]) -> RuleTable {
    let mut rules = RuleTable::default();
    for &column in columns {
        rules.toggle(column, rule);
    }
    rules
}

#[test]
fn palm_mute_columns_become_mute_down() {
    let rules = columns(Rule::PalmMute, &[1, 2]);
    assert_eq!(
        articulations("o3 l8 e f+ g a", &rules),
        vec![SusDown, MuteDown, MuteDown, SusDown]
    );
}

#[test]
fn palm_mute_keeps_the_economy_stroke() {
    // E2 F#2 G2 は同じ弦で D U D、A2 は高い弦へ移るので D。
    let mut rules = columns(Rule::PalmMute, &[0, 1, 2, 3]);
    rules.toggle_row(RowRule::EconomyPicking);
    assert_eq!(
        articulations("o3 l8 e f+ g a", &rules),
        vec![MuteDown, MuteUp, MuteDown, MuteDown]
    );
}

#[test]
fn pinch_harmonic_columns_become_ph_for_both_strokes() {
    let mut rules = columns(Rule::PinchHarmonic, &[0, 1]);
    rules.toggle_row(RowRule::EconomyPicking);
    assert_eq!(
        articulations("o3 l8 e f+ g a", &rules),
        vec![PinchHarmonic, PinchHarmonic, SusDown, SusDown]
    );
}

#[test]
fn notes_without_a_sample_stay_sus_down() {
    // o6 g = 79 は 30〜76 の外。
    assert_eq!(notes("o6 g")[0].pitch, 79);
    for rule in [Rule::PalmMute, Rule::PinchHarmonic] {
        assert_eq!(articulations("o6 g", &columns(rule, &[0])), vec![SusDown]);
    }
    assert_eq!(notes("o2 f+")[0].pitch, 30);
    assert_eq!(
        articulations("o2 f+", &columns(Rule::PalmMute, &[0])),
        vec![MuteDown]
    );
}

#[test]
fn hammer_pull_notes_stay_as_they_are() {
    // 自動ハンマリングで 2 音目が H/P になる列に PalmMute を ON にしても H/P のまま。
    let mut rules = columns(Rule::PalmMute, &[1]);
    rules.toggle_row(RowRule::AutoHammerPull);
    assert_eq!(articulations("o3 l8 e f+", &rules), vec![SusDown, HammerOn]);
}

#[test]
fn chord_columns_are_voiced_note_by_note() {
    assert_eq!(
        articulations("o3 l8 'egb'", &columns(Rule::PalmMute, &[0])),
        vec![MuteDown, MuteDown, MuteDown]
    );
}
