use std::collections::BTreeSet;

use rand::{rngs::StdRng, SeedableRng};

use super::*;
use crate::{convert, notes_from_events, RowRule, Rule, RuleTable};

/// 1 オクターブを 2 往復する 16 分音符 32 音（120 BPM）。
const OCTAVE_RUN: &str =
    "t120 o4 l16 c d e f g a b < c > b a g f e d c d e f g a b < c > b a g f e d c d e f";

const CHORDS: &str = "o4 l8 c 'ceg' d 'ceg' e";

/// 単音 16 分・和音混じり・連打・休符入り。
const MMLS: [&str; 4] = [OCTAVE_RUN, CHORDS, "o4 l8 c c c c", "o3 l8 e r e r"];

fn raw(mml: &str) -> Vec<TimedMidiEvent> {
    cmrt_chord::timed_performance(mml).unwrap().events
}

fn notes(mml: &str) -> Vec<Note> {
    notes_from_events(&raw(mml))
}

fn values(events: &[TimedMidiEvent], controller: u8) -> Vec<(f64, u8)> {
    events
        .iter()
        .filter(|e| e.message[0] & 0xF0 == 0xB0 && e.message[1] == controller)
        .map(|e| (e.seconds, e.message[2]))
        .collect()
}

fn is_release_cc(e: &TimedMidiEvent) -> bool {
    e.message[0] & 0xF0 == 0xB0 && matches!(e.message[1], RELEASE_SHAPE_CC | RELEASE_LEVEL_CC)
}

fn column_count(notes: &[Note]) -> usize {
    notes.last().map_or(0, |n| n.column + 1)
}

#[test]
fn each_column_gets_one_pair_and_the_end_resets_to_the_defaults() {
    let notes = notes(OCTAVE_RUN);
    let out = release_events(&notes, &mut StdRng::seed_from_u64(1));
    let shape = values(&out, RELEASE_SHAPE_CC);
    let level = values(&out, RELEASE_LEVEL_CC);
    assert_eq!(column_count(&notes), 32);
    assert_eq!(shape.len(), 33);
    assert_eq!(level.len(), 33);

    let end = notes.iter().map(|n| n.off_seconds).fold(0.0, f64::max);
    assert_eq!(shape.last(), Some(&(end, RELEASE_SHAPE_DEFAULT)));
    assert_eq!(level.last(), Some(&(end, RELEASE_LEVEL_DEFAULT)));

    for (note, (&(shape_at, _), &(level_at, _))) in notes.iter().zip(shape.iter().zip(&level)) {
        assert_eq!(shape_at, note.on_seconds);
        assert_eq!(level_at, note.on_seconds);
    }
}

#[test]
fn values_stay_in_the_ranges_and_spread_over_the_bands() {
    let notes = notes(OCTAVE_RUN);
    let out = release_events(&notes, &mut StdRng::seed_from_u64(1));
    let shape = values(&out, RELEASE_SHAPE_CC);
    let level = values(&out, RELEASE_LEVEL_CC);
    let shape = &shape[..shape.len() - 1];
    let level = &level[..level.len() - 1];

    assert!(shape.iter().all(|(_, v)| (2..=63).contains(v)));
    assert!(level.iter().all(|(_, v)| (95..=127).contains(v)));
    let bands: BTreeSet<u8> = shape.iter().map(|(_, v)| v / 16).collect();
    assert!(bands.len() >= 3, "bands {bands:?}");
    let levels: BTreeSet<u8> = level.iter().map(|(_, v)| *v).collect();
    assert!(levels.len() >= 5, "levels {levels:?}");
}

#[test]
fn a_chord_column_gets_one_pair_at_its_earliest_note_on() {
    let notes = notes(CHORDS);
    let out = release_events(&notes, &mut StdRng::seed_from_u64(1));
    let shape = values(&out, RELEASE_SHAPE_CC);
    let level = values(&out, RELEASE_LEVEL_CC);
    assert_eq!(column_count(&notes), 5);
    assert!(notes.len() > 5);
    assert_eq!(shape.len(), 6);
    assert_eq!(level.len(), 6);
    for column in 0..5 {
        let earliest = notes
            .iter()
            .filter(|n| n.column == column)
            .map(|n| n.on_seconds)
            .fold(f64::INFINITY, f64::min);
        assert_eq!(shape[column].0, earliest);
        assert_eq!(level[column].0, earliest);
    }
}

#[test]
fn the_same_seed_gives_the_same_result_and_another_seed_differs() {
    let notes = notes(OCTAVE_RUN);
    let a = release_events(&notes, &mut StdRng::seed_from_u64(1));
    let b = release_events(&notes, &mut StdRng::seed_from_u64(1));
    let c = release_events(&notes, &mut StdRng::seed_from_u64(2));
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn no_notes_give_no_events() {
    assert!(seeded_release_events(&[]).is_empty());
}

/// なし / eco / auto / 列の H/P / 汚し / 汚し + eco + 列の H/P / 汚し + auto + 列の H/P
/// （`eco` と `auto` は排他なので、汚しと重ねる表は片方ずつ）。
fn rule_sets() -> Vec<RuleTable> {
    let row = |rows: &[RowRule]| {
        let mut rules = RuleTable::default();
        for &rule in rows {
            rules.toggle_row(rule);
        }
        rules
    };
    let with_columns = |mut rules: RuleTable| {
        rules.toggle(1, Rule::HammerPull);
        rules.toggle(2, Rule::HammerPull);
        rules
    };
    vec![
        RuleTable::default(),
        row(&[RowRule::EconomyPicking]),
        row(&[RowRule::AutoHammerPull]),
        with_columns(RuleTable::default()),
        row(&[RowRule::Humanize]),
        with_columns(row(&[RowRule::Humanize, RowRule::EconomyPicking])),
        with_columns(row(&[RowRule::Humanize, RowRule::AutoHammerPull])),
    ]
}

fn with_release(rules: &RuleTable) -> RuleTable {
    let mut rules = rules.clone();
    rules.toggle_row(RowRule::HumanizeRelease);
    rules
}

#[test]
fn the_rule_only_adds_cc24_and_cc25() {
    for mml in MMLS {
        for rules in rule_sets() {
            let without = convert(&raw(mml), &rules);
            let with = convert(&raw(mml), &with_release(&rules));
            assert!(!without.iter().any(is_release_cc), "{mml} {rules:?}");
            let stripped: Vec<TimedMidiEvent> =
                with.iter().filter(|e| !is_release_cc(e)).copied().collect();
            assert_eq!(stripped, without, "{mml} {rules:?}");
            let columns = column_count(&notes(mml));
            assert_eq!(
                values(&with, RELEASE_SHAPE_CC).len(),
                columns + 1,
                "{mml} {rules:?}"
            );
        }
    }
}

/// 演奏音だけ（KS と CC を除く）を音へまとめ直す。汚しでずれた和音は別の列に割れるので、列は元の MML から数える。
fn played_notes(out: &[TimedMidiEvent]) -> Vec<Note> {
    let played: Vec<TimedMidiEvent> = out
        .iter()
        .filter(|e| e.message[0] & 0xF0 != 0xB0 && e.message[1] >= 30)
        .copied()
        .collect();
    notes_from_events(&played)
}

#[test]
fn each_pair_comes_before_its_note_on_and_before_the_note_offs_of_its_column() {
    for mml in MMLS {
        let written = notes(mml);
        for rules in rule_sets() {
            let out = convert(&raw(mml), &with_release(&rules));
            let played = played_notes(&out);
            assert_eq!(played.len(), written.len());
            let shape_at: Vec<usize> = out
                .iter()
                .enumerate()
                .filter(|(_, e)| e.message[0] & 0xF0 == 0xB0 && e.message[1] == RELEASE_SHAPE_CC)
                .map(|(i, _)| i)
                .collect();
            let mut first = 0;
            assert_eq!(shape_at.len(), column_count(&written) + 1);
            for (column, &i) in shape_at.iter().take(shape_at.len() - 1).enumerate() {
                let size = written.iter().filter(|n| n.column == column).count();
                let members = &played[first..first + size];
                first += size;
                let seconds = out[i].seconds;
                let earliest = members
                    .iter()
                    .map(|n| n.on_seconds)
                    .fold(f64::INFINITY, f64::min);
                assert_eq!(seconds, earliest, "{mml} {rules:?} column {column}");
                let note_on = out
                    .iter()
                    .position(|e| {
                        e.seconds == seconds
                            && e.message[0] & 0xF0 == 0x90
                            && e.message[2] != 0
                            && e.message[1] >= 30
                    })
                    .unwrap_or_else(|| panic!("{mml} {rules:?}: no note on at {seconds}"));
                assert!(i < note_on, "{mml} {rules:?} at {seconds}");
                assert!(out[i + 1].message[1] == RELEASE_LEVEL_CC && i + 1 < note_on);
                assert!(members.iter().all(|n| seconds <= n.off_seconds));
            }
        }
    }
}

#[test]
fn shape_names_follow_the_four_bands() {
    let names: Vec<&str> = [2, 15, 16, 31, 32, 47, 48, 63]
        .into_iter()
        .map(release_shape_name)
        .collect();

    assert_eq!(
        names,
        [
            "Basic",
            "Basic",
            "Hard",
            "Hard",
            "Agressive",
            "Agressive",
            "Agressive2",
            "Agressive2"
        ]
    );
}
