use super::*;
use crate::RowRule;

use Articulation::{HammerOn, PullOff, SusDown, SusUp};

/// 0.5 秒ずつ並んだ単音の列（velocity 100）。
fn melody(pitches: &[u8]) -> Vec<Note> {
    pitches
        .iter()
        .enumerate()
        .map(|(column, &pitch)| Note {
            on_seconds: column as f64 * 0.5,
            off_seconds: column as f64 * 0.5 + 0.5,
            channel: 0,
            pitch,
            velocity: 100,
            column,
        })
        .collect()
}

fn rules(rows: &[RowRule]) -> RuleTable {
    let mut rules = RuleTable::default();
    for &row in rows {
        rules.toggle_row(row);
    }
    rules
}

fn strokes(notes: &[Note], rules: &RuleTable) -> Vec<Articulation> {
    articulate(notes, rules)
        .iter()
        .map(|a| a.articulation)
        .collect()
}

#[test]
fn ascending_three_notes_per_string_sweeps_down_into_the_next_string() {
    // 全音音階は 3 音で 5 半音を超えるので、3 音ごとに弦を移る。
    let n = melody(&[40, 42, 44, 46, 48, 50, 52]);
    assert_eq!(
        strokes(&n, &rules(&[RowRule::EconomyPicking])),
        vec![SusDown, SusUp, SusDown, SusDown, SusUp, SusDown, SusDown]
    );
}

#[test]
fn descending_pulls_off_after_a_down_stroke_to_leave_the_string_on_an_up_stroke() {
    let n = melody(&[52, 50, 48, 46, 44, 42, 40]);
    assert_eq!(
        strokes(&n, &rules(&[RowRule::EconomyPicking])),
        vec![SusDown, SusUp, PullOff, SusDown, SusUp, PullOff, SusDown]
    );
}

#[test]
fn a_descending_pentatonic_goes_down_pull_off_down() {
    let n = melody(&[60, 57, 55, 52, 50, 48, 45, 43]);
    assert_eq!(
        strokes(&n, &rules(&[RowRule::EconomyPicking])),
        vec![SusDown, PullOff, SusDown, PullOff, SusDown, PullOff, SusDown, SusUp]
    );
}

#[test]
fn descending_stays_alternate_when_the_string_has_no_note_to_pull_off_from() {
    // 1 弦 1 音ずつ。
    let n = melody(&[60, 53, 46]);
    assert_eq!(
        strokes(&n, &rules(&[RowRule::EconomyPicking])),
        vec![SusDown, SusUp, SusDown]
    );
}

#[test]
fn with_auto_hammer_pull_only_the_first_note_of_each_string_is_picked() {
    // 弦の頭だけピッキング。上行はダウンのスイープ、下行は H/P の後なのでダウンで入る。
    let n = melody(&[40, 42, 44, 46, 44, 42, 40]);
    assert_eq!(
        strokes(
            &n,
            &rules(&[RowRule::EconomyPicking, RowRule::AutoHammerPull])
        ),
        vec![SusDown, HammerOn, HammerOn, SusDown, PullOff, PullOff, SusDown]
    );
}

#[test]
fn only_the_apex_keeps_its_velocity_while_economy_picking_is_on() {
    let n = melody(&[40, 42, 44, 42, 40, 42]);
    let out = articulate(&n, &rules(&[RowRule::EconomyPicking]));

    let accents: Vec<bool> = out.iter().map(|a| a.accent).collect();
    assert_eq!(accents, vec![false, false, true, false, false, false]);
    let velocities: Vec<u8> = out.iter().map(|a| a.velocity).collect();
    assert_eq!(velocities, vec![75, 75, 100, 75, 75, 75]);
}

#[test]
fn off_keeps_down_strokes_and_the_original_velocity() {
    let n = melody(&[40, 42, 44, 42]);
    let out = articulate(&n, &RuleTable::default());

    assert!(out.iter().all(|a| a.articulation == SusDown && !a.accent));
    assert!(out.iter().all(|a| a.velocity == 100));
}

#[test]
fn a_chord_column_stays_down_and_restarts_the_strokes() {
    let mut n = melody(&[40, 42, 45, 47]);
    // 2 列目を 42 + 45 の和音にし、以降の列を詰める。
    n[2].on_seconds = n[1].on_seconds;
    n[2].column = 1;
    n[3].column = 2;
    assert_eq!(
        strokes(&n, &rules(&[RowRule::EconomyPicking])),
        vec![SusDown, SusDown, SusDown, SusDown]
    );
}
