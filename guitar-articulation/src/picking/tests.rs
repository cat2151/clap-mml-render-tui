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
fn only_the_head_and_the_apex_keep_their_velocity_while_economy_picking_is_on() {
    let n = melody(&[40, 42, 44, 42, 40, 42]);
    let out = articulate(&n, &rules(&[RowRule::EconomyPicking]));

    let accents: Vec<bool> = out.iter().map(|a| a.accent).collect();
    assert_eq!(accents, vec![true, false, true, false, false, false]);
    let velocities: Vec<u8> = out.iter().map(|a| a.velocity).collect();
    assert_eq!(velocities, vec![100, 75, 100, 75, 75, 75]);
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

#[test]
fn humanize_alone_also_lowers_the_unaccented_picks() {
    let n = melody(&[40, 42, 44, 42, 40, 42]);
    let out = articulate(&n, &rules(&[RowRule::Humanize]));

    let velocities: Vec<u8> = out.iter().map(|a| a.velocity).collect();
    assert_eq!(velocities, vec![100, 75, 100, 75, 75, 75]);
}

#[test]
fn legato_mute_and_pinch_keep_their_velocity_under_dynamics() {
    let n = melody(&[40, 42, 44, 42, 40, 42]);
    let mut table = rules(&[RowRule::EconomyPicking, RowRule::Humanize]);
    table.toggle(1, crate::Rule::HammerPull);
    table.toggle(3, crate::Rule::PalmMute);
    table.toggle(4, crate::Rule::PinchHarmonic);
    let out = articulate(&n, &table);

    let got: Vec<(Articulation, u8)> = out.iter().map(|a| (a.articulation, a.velocity)).collect();
    assert_eq!(got[1], (HammerOn, 100));
    assert!(
        matches!(got[3].0, Articulation::MuteDown | Articulation::MuteUp),
        "{got:?}"
    );
    assert_eq!(got[3].1, 100);
    assert_eq!(got[4], (Articulation::PinchHarmonic, 100));
    assert_eq!(got[5].1, 75, "{got:?}");
}

#[test]
fn auto_hammer_pull_alternates_only_the_picked_notes() {
    // C D E F G A G F E D C
    let n = melody(&[60, 62, 64, 65, 67, 69, 67, 65, 64, 62, 60]);
    assert_eq!(
        strokes(&n, &rules(&[RowRule::AutoHammerPull])),
        vec![
            SusDown, HammerOn, HammerOn, SusUp, HammerOn, SusDown, PullOff, PullOff, SusUp,
            PullOff, SusDown
        ]
    );
}

#[test]
fn auto_hammer_pull_restarts_down_after_a_chord() {
    let mut n = melody(&[40, 45, 42, 45, 50]);
    // 3 列目を 42 + 45 の和音にし、以降の列を詰める。
    n[3].on_seconds = n[2].on_seconds;
    n[3].column = 2;
    n[4].column = 3;
    assert_eq!(
        strokes(&n, &rules(&[RowRule::AutoHammerPull])),
        vec![SusDown, SusUp, SusDown, SusDown, SusDown]
    );
}
