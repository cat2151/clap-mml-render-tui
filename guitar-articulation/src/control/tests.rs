use super::*;
use crate::{articulate, notes_from_events, Rule, RuleTable};

fn events_for(mml: &str, rules: &RuleTable) -> Vec<TimedMidiEvent> {
    let notes = notes_from_events(&cmrt_chord::timed_performance(mml).unwrap().events);
    let articulations: Vec<Articulation> = articulate(&notes, rules)
        .iter()
        .map(|a| a.articulation)
        .collect();
    control_events(&notes, &articulations, rules)
}

fn slide_on(columns: &[usize]) -> RuleTable {
    let mut rules = RuleTable::default();
    for &column in columns {
        rules.toggle(column, Rule::Slide);
    }
    rules
}

#[test]
fn a_slide_sends_its_width_then_resets_at_the_end() {
    // E2 → G2 は +3。l8 = 0.25 秒。
    let out = events_for("o3 l8 e g", &slide_on(&[1]));
    assert_eq!(
        out,
        vec![
            control_change(0.25, 0, SLIDE_WIDTH_CC, 40),
            control_change(0.5, 0, SLIDE_WIDTH_CC, SLIDE_WIDTH_CC_DEFAULT),
        ]
    );
}

#[test]
fn width_values_are_the_middle_of_each_sfz_range() {
    assert_eq!(slide_width_value(1), 8);
    assert_eq!(slide_width_value(2), 24);
    assert_eq!(slide_width_value(SLIDE_MAX_SEMITONES), 104);
}

#[test]
fn no_slide_no_control_change() {
    assert!(events_for("o3 l8 e g a", &RuleTable::default()).is_empty());
    // 範囲外（+11）で滑らなければ戻しも送らない。
    assert!(events_for("o3 l8 c b", &slide_on(&[1])).is_empty());
    let mut choke = RuleTable::default();
    choke.toggle(1, Rule::Choke);
    assert!(events_for("o3 l8 e g", &choke).is_empty());
}

fn vibrato_on(columns: &[usize]) -> RuleTable {
    let mut rules = RuleTable::default();
    for &column in columns {
        rules.toggle(column, Rule::Vibrato);
    }
    rules
}

fn vibrato(seconds: f64, value: u8) -> TimedMidiEvent {
    control_change(seconds, 0, VIBRATO_DEPTH_CC, value)
}

#[test]
fn a_vibrato_column_sends_depth_at_on_and_zero_at_off_then_resets() {
    let out = events_for("o3 l8 e g a", &vibrato_on(&[1]));
    assert_eq!(
        out,
        vec![
            vibrato(0.25, VIBRATO_DEPTH),
            vibrato(0.5, 0),
            vibrato(0.75, 0)
        ]
    );
}

#[test]
fn a_vibrato_chord_sends_once_per_column() {
    let out = events_for("o3 l8 e 'gb<d' a", &vibrato_on(&[1]));
    assert_eq!(
        out,
        vec![
            vibrato(0.25, VIBRATO_DEPTH),
            vibrato(0.5, 0),
            vibrato(0.75, 0)
        ]
    );
}

#[test]
fn back_to_back_vibrato_columns_reset_before_the_next_depth() {
    let out = events_for("o3 l8 e g a", &vibrato_on(&[1, 2]));
    assert_eq!(
        out,
        vec![
            vibrato(0.25, VIBRATO_DEPTH),
            vibrato(0.5, 0),
            vibrato(0.5, VIBRATO_DEPTH),
            vibrato(0.75, 0),
            vibrato(0.75, 0)
        ]
    );
}

#[test]
fn no_vibrato_no_cc20() {
    let out = events_for("o3 l8 e g a e", &slide_on(&[1, 2, 3]));
    assert!(out.iter().all(|e| e.message[1] != VIBRATO_DEPTH_CC));
}
