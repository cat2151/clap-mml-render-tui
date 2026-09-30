use std::collections::BTreeSet;

use rand::{rngs::StdRng, SeedableRng};

use super::*;
use crate::{articulate, notes_from_events, RowRule, RuleTable};

mod events;
mod rule_off;

/// 1 オクターブを 2 往復する 16 分音符 32 音（120 BPM）。アクセントは頭と、頂点の上の c の 2 か所。
const OCTAVE_RUN: &str =
    "t120 o4 l16 c d e f g a b < c > b a g f e d c d e f g a b < c > b a g f e d c d e f";

fn notes(mml: &str) -> Vec<Note> {
    notes_from_events(&cmrt_chord::timed_performance(mml).unwrap().events)
}

fn rules(rows: &[RowRule]) -> RuleTable {
    let mut rules = RuleTable::default();
    for &row in rows {
        rules.toggle_row(row);
    }
    rules
}

/// 汚しの ON に `rows` を足した表で [`articulate`] してから汚す。
fn run(mml: &str, rows: &[RowRule], seed: u64) -> (Vec<Note>, Vec<Articulated>, Vec<Humanized>) {
    let notes = notes(mml);
    let rows: Vec<RowRule> = rows.iter().copied().chain([RowRule::Humanize]).collect();
    let articulated = articulate(&notes, &rules(&rows));
    let humanized = humanize(&notes, &articulated, &mut StdRng::seed_from_u64(seed));
    (notes, articulated, humanized)
}

/// 列 c のどの on も、列 c+1 のどの on より前か。
fn columns_stay_in_order(notes: &[Note], humanized: &[Humanized]) -> bool {
    let last_column = notes.last().map_or(0, |n| n.column);
    (0..last_column).all(|c| {
        let latest = notes
            .iter()
            .zip(humanized)
            .filter(|(n, _)| n.column == c)
            .map(|(_, h)| h.on_seconds)
            .fold(f64::MIN, f64::max);
        let earliest = notes
            .iter()
            .zip(humanized)
            .filter(|(n, _)| n.column == c + 1)
            .map(|(_, h)| h.on_seconds)
            .fold(f64::MAX, f64::min);
        latest < earliest
    })
}

#[test]
fn the_same_seed_gives_the_same_result_and_another_seed_differs() {
    let (_, _, a) = run(OCTAVE_RUN, &[], 1);
    let (_, _, b) = run(OCTAVE_RUN, &[], 1);
    let (_, _, c) = run(OCTAVE_RUN, &[], 2);
    assert_eq!(a, b);
    assert_ne!(a, c);
}

#[test]
fn on_shifts_stay_within_the_width_and_keep_the_column_order() {
    for seed in 0..20 {
        let (notes, _, humanized) = run(OCTAVE_RUN, &[], seed);
        assert_eq!(notes.len(), 32);
        for (note, h) in notes.iter().zip(&humanized) {
            assert!((h.on_seconds - note.on_seconds).abs() <= HUMANIZE_ON_SECONDS + 1e-12);
            assert!(h.on_seconds >= 0.0);
            assert!(h.off_seconds > h.on_seconds);
        }
        assert!(columns_stay_in_order(&notes, &humanized), "seed {seed}");
    }
    // ジャストのままの音ばかりではない。
    let (notes, _, humanized) = run(OCTAVE_RUN, &[], 0);
    let shifted = notes
        .iter()
        .zip(&humanized)
        .filter(|(n, h)| (n.on_seconds - h.on_seconds).abs() > 1e-6)
        .count();
    assert!(shifted > 16, "shifted {shifted}");
}

#[test]
fn a_repeated_pitch_is_released_before_its_next_attack() {
    for seed in 0..20 {
        let (_, _, humanized) = run("t120 o4 l16 c c c c", &[], seed);
        for pair in humanized.windows(2) {
            assert!(pair[0].off_seconds <= pair[1].on_seconds, "seed {seed}");
        }
    }
}

#[test]
fn unaccented_notes_are_quieter_than_accents_and_spread_out() {
    let (_, articulated, humanized) = run(OCTAVE_RUN, &[], 0);
    let accents: Vec<bool> = articulated.iter().map(|a| a.accent).collect();
    assert_eq!(accents.iter().filter(|&&a| a).count(), 3);
    let accent_min = humanized
        .iter()
        .zip(&accents)
        .filter(|(_, &a)| a)
        .map(|(h, _)| h.velocity)
        .min()
        .unwrap();
    let mut unaccented: Vec<u8> = humanized
        .iter()
        .zip(&accents)
        .filter(|(_, &a)| !a)
        .map(|(h, _)| h.velocity)
        .collect();
    unaccented.sort_unstable();
    assert!(unaccented[unaccented.len() / 2] < accent_min);
    assert!(unaccented.iter().collect::<BTreeSet<_>>().len() >= 5);
    // 強弱は articulate が決めたものを散らすだけ。
    assert!(humanized.iter().zip(&articulated).all(|(h, a)| {
        let shift = i32::from(h.velocity) - i32::from(a.velocity);
        (-VELOCITY_SPREAD..=VELOCITY_SPREAD).contains(&shift)
    }));
}

#[test]
fn only_picked_notes_carry_picking_noise_values_within_range() {
    let (_, articulated, humanized) = run(OCTAVE_RUN, &[RowRule::AutoHammerPull], 0);
    let mut picked = Vec::new();
    for (a, h) in articulated.iter().zip(&humanized) {
        match a.articulation {
            Articulation::HammerOn | Articulation::PullOff => assert_eq!(h.picking, None),
            Articulation::SusDown | Articulation::SusUp => {
                let [cc30, cc31] = h.picking.expect("picked note has CC values");
                assert!(PICKING_CC30_RANGE.contains(&cc30));
                assert!(PICKING_CC31_RANGE.contains(&cc31));
                picked.push([cc30, cc31]);
            }
            _ => {}
        }
    }
    assert!(articulated.iter().any(|a| matches!(
        a.articulation,
        Articulation::HammerOn | Articulation::PullOff
    )));
    assert!(picked.iter().collect::<BTreeSet<_>>().len() >= 2);
}

#[test]
fn chord_notes_do_not_overtake_the_neighbouring_columns() {
    for seed in 0..20 {
        let (notes, _, humanized) = run("t120 o4 l16 c 'ceg' d 'ceg' e", &[], seed);
        assert_eq!(notes.len(), 9);
        assert!(columns_stay_in_order(&notes, &humanized), "seed {seed}");
    }
}
