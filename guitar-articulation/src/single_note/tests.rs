use super::*;
use crate::ui::{ROW_RULE_ROWS, RULE_ROWS};
use crate::{
    articulate, convert, notes_from_events, RowRule, Rule, RuleTable, DEFAULT_MML,
    KEYSWITCH_VELOCITY,
};

const EPSILON: f64 = 1e-9;

fn from_mml(mml: &str) -> Vec<TimedMidiEvent> {
    cmrt_chord::timed_performance(mml).unwrap().events
}

fn column_of(mml: &str, rules: &RuleTable, column: usize, take: Take) -> Vec<TimedMidiEvent> {
    let notes = notes_from_events(&from_mml(mml));
    let articulated = articulate(&notes, rules);
    column_events(&notes, &articulated, rules, column, take)
}

fn is_keyswitch_on(e: &TimedMidiEvent) -> bool {
    e.message[0] & 0xF0 == 0x90
        && e.message[2] != 0
        && Articulation::from_keyswitch(e.message[1]).is_some()
}

/// `(秒, message)` を、秒は誤差込みで比べる。
fn assert_events(actual: &[TimedMidiEvent], expected: &[(f64, [u8; 3])]) {
    let actual_messages: Vec<[u8; 3]> = actual.iter().map(|e| e.message).collect();
    let expected_messages: Vec<[u8; 3]> = expected.iter().map(|(_, m)| *m).collect();
    assert_eq!(actual_messages, expected_messages);
    for (a, (seconds, _)) in actual.iter().zip(expected) {
        assert!((a.seconds - seconds).abs() < EPSILON, "{actual:?}");
    }
}

fn hammer_on_cde() -> RuleTable {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::HammerPull);
    rules.toggle(2, Rule::HammerPull);
    rules
}

const SUS_DOWN: u8 = 17;
const HAMMER_ON: u8 = 26;
const E4: u8 = 64;
const C4: u8 = 60;

#[test]
fn latched_hammer_on_column_presses_its_own_keyswitch() {
    let out = column_of("l16cde", &hammer_on_cde(), 2, Take::Converted);

    assert_events(
        &out,
        &[
            (0.0, [0x90, SUS_DOWN, KEYSWITCH_VELOCITY]),
            (0.0, [0x90, HAMMER_ON, KEYSWITCH_VELOCITY]),
            (0.0, [0x90, E4, 127]),
            (0.125, [0x80, SUS_DOWN, 0]),
            (0.125, [0x80, HAMMER_ON, 0]),
            (0.125, [0x80, E4, 0]),
            (0.125, [0x90, SUS_DOWN, KEYSWITCH_VELOCITY]),
            (0.175, [0x80, SUS_DOWN, 0]),
        ],
    );
}

#[test]
fn default_articulation_column_has_only_the_head_sus_down() {
    let out = column_of("l16cde", &hammer_on_cde(), 0, Take::Converted);

    assert_events(
        &out,
        &[
            (0.0, [0x90, SUS_DOWN, KEYSWITCH_VELOCITY]),
            (0.0, [0x90, C4, 127]),
            (0.125, [0x80, SUS_DOWN, 0]),
            (0.125, [0x80, C4, 0]),
        ],
    );
}

#[test]
fn plain_has_no_keyswitch_and_keeps_the_mml_velocity() {
    let mut rules = hammer_on_cde();
    rules.toggle_row(crate::RowRule::EconomyPicking);
    let notes = notes_from_events(&from_mml("l16cde"));
    let out = column_of("l16cde", &rules, 2, Take::Plain);

    assert_events(
        &out,
        &[(0.0, [0x90, E4, notes[2].velocity]), (0.125, [0x80, E4, 0])],
    );
}

#[test]
fn chord_column_has_every_note() {
    let out = column_of("'ceg' d", &RuleTable::default(), 0, Take::Plain);

    let pitches: Vec<u8> = out
        .iter()
        .filter(|e| e.message[0] & 0xF0 == 0x90)
        .map(|e| e.message[1])
        .collect();
    assert_eq!(pitches.len(), 3, "{out:?}");
}

#[test]
fn out_of_range_column_is_empty() {
    assert!(column_of("l16cde", &hammer_on_cde(), 3, Take::Converted).is_empty());
    assert!(column_events(&[], &[], &RuleTable::default(), 0, Take::Converted).is_empty());
}

/// 全列で全部の列ルールを ON にし、汚し 2 つ以外の全部の行ルールも ON にした表。
/// 汚しは 2 つとも 1 音の試聴に掛からない（時刻と velocity がフレーズの演奏と一致しない。
/// リリースの CC はフレーズの列ごとの値で、1 音の試聴には送らない）ので外す。
/// ピックスクレイプは列の全部の音を上書きして他の列ルールが効かなくなるので外す（`scratch/tests.rs` で見る）。
fn all_rules_on(columns: usize) -> RuleTable {
    let mut rules = RuleTable::default();
    for column in 0..columns {
        for (rule, _, _) in RULE_ROWS {
            if rule == Rule::PickScratch {
                continue;
            }
            rules.toggle(column, rule);
        }
    }
    for (rule, _, _) in ROW_RULE_ROWS {
        if matches!(rule, RowRule::Humanize | RowRule::HumanizeRelease) {
            continue;
        }
        rules.toggle_row(rule);
    }
    rules
}

/// `events` の `index` 番より前で最後に押された KS。
fn last_keyswitch_before(events: &[TimedMidiEvent], index: usize) -> Option<u8> {
    events[..index]
        .iter()
        .rev()
        .find(|e| is_keyswitch_on(e))
        .map(|e| e.message[1])
}

fn note_on_index(events: &[TimedMidiEvent], note: &Note, seconds: f64) -> usize {
    events
        .iter()
        .position(|e| {
            e.message[0] == 0x90 | note.channel
                && e.message[1] == note.pitch
                && e.message[2] != 0
                && (e.seconds - seconds).abs() < EPSILON
        })
        .unwrap_or_else(|| panic!("note on of {note:?} at {seconds} not in {events:?}"))
}

fn control_numbers_at(events: &[TimedMidiEvent], seconds: f64) -> Vec<u8> {
    let mut numbers: Vec<u8> = events
        .iter()
        .filter(|e| e.message[0] & 0xF0 == 0xB0 && (e.seconds - seconds).abs() < EPSILON)
        .map(|e| e.message[1])
        .collect();
    numbers.sort_unstable();
    numbers.dedup();
    numbers
}

/// `convert` に足された物（velocity・KS・音の頭の CC）を、1 音ぶんの列が取りこぼしていないか。
#[test]
fn every_column_matches_the_converted_phrase() {
    for mml in [
        DEFAULT_MML,
        "o3 l16 e f+ g a b a g f+",
        "o3 l8 'egb' e f+ g 'egb' e",
    ] {
        let plain = from_mml(mml);
        let notes = notes_from_events(&plain);
        let columns = notes.last().map_or(0, |n| n.column + 1);
        for rules in [RuleTable::default(), all_rules_on(columns)] {
            let converted = convert(&plain, &rules);
            let articulated = articulate(&notes, &rules);
            for column in 0..columns {
                let single = column_events(&notes, &articulated, &rules, column, Take::Converted);
                let context = format!("mml={mml} rules={} column={column}", rules.to_json());
                for note in notes.iter().filter(|n| n.column == column) {
                    let whole_index = note_on_index(&converted, note, note.on_seconds);
                    let single_index = note_on_index(&single, note, 0.0);
                    assert_eq!(
                        single[single_index].message[2], converted[whole_index].message[2],
                        "velocity: {context}"
                    );
                    assert_eq!(
                        last_keyswitch_before(&single, single_index),
                        last_keyswitch_before(&converted, whole_index),
                        "keyswitch: {context}"
                    );
                    let whole_cc = control_numbers_at(&converted, note.on_seconds);
                    let single_cc = control_numbers_at(&single, 0.0);
                    assert!(
                        whole_cc.iter().all(|cc| single_cc.contains(cc)),
                        "cc {whole_cc:?} not in {single_cc:?}: {context}"
                    );
                }
            }
        }
    }
}

#[test]
fn a_slide_column_alone_keeps_its_width_from_the_previous_column() {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::Slide);
    let single = column_of("o3 l8 e g", &rules, 1, Take::Converted);

    let width = single
        .iter()
        .position(|e| e.message == [0xB0, 26, 40])
        .unwrap_or_else(|| panic!("+3 半音の CC26 が無い: {single:?}"));
    let note_on = single
        .iter()
        .position(|e| e.message[0] == 0x90 && e.message[1] == 43 && e.message[2] != 0)
        .unwrap();
    assert!(width < note_on, "{single:?}");
    assert!(column_of("o3 l8 e g", &rules, 1, Take::Plain)
        .iter()
        .all(|e| e.message[0] & 0xF0 != 0xB0));
}
