use super::*;
use crate::{articulate, column_events, convert, notes_from_events, RowRule, Take};

use Articulation::{SusDown, UnisonBendManual};

fn raw(mml: &str) -> Vec<TimedMidiEvent> {
    cmrt_chord::timed_performance(mml).unwrap().events
}

/// (on 秒, off 秒, 音高) の音を channel 0 で並べる。
fn raw_notes(notes: &[(f64, f64, u8)]) -> Vec<TimedMidiEvent> {
    let mut out = Vec::new();
    for &(on, off, pitch) in notes {
        out.push(TimedMidiEvent {
            seconds: on,
            message: [0x90, pitch, 100],
        });
        out.push(TimedMidiEvent {
            seconds: off,
            message: [0x80, pitch, 0],
        });
    }
    cmrt_midi_filter::sort_for_playback(&mut out);
    out
}

fn manual_on(columns: &[usize]) -> RuleTable {
    let mut rules = RuleTable::default();
    for &column in columns {
        rules.toggle(column, Rule::UnisonBendManual);
    }
    rules
}

fn articulations(events: &[TimedMidiEvent], rules: &RuleTable) -> Vec<Articulation> {
    articulate(&notes_from_events(events), rules)
        .iter()
        .map(|a| a.articulation)
        .collect()
}

fn bends(events: &[TimedMidiEvent]) -> Vec<(f64, u16)> {
    events
        .iter()
        .filter(|e| e.message[0] & 0xF0 == 0xE0)
        .map(|e| {
            let value = u16::from(e.message[1]) | (u16::from(e.message[2]) << 7);
            (e.seconds, value)
        })
        .collect()
}

#[test]
fn the_column_uses_keyswitch_95() {
    assert_eq!(UnisonBendManual.keyswitch(), 95);
    let out = convert(&raw("o5 l4 e g2 a"), &manual_on(&[1]));
    assert!(out
        .iter()
        .any(|e| e.seconds == 0.5 && e.message[0] == 0x90 && e.message[1] == 95));
    assert_eq!(
        articulations(&raw("o5 l4 e g2 a"), &manual_on(&[1])),
        vec![SusDown, UnisonBendManual, SusDown]
    );
}

#[test]
fn notes_outside_c4_to_c6_stay_sus_down() {
    for (pitch, expected) in [
        (59, SusDown),
        (60, UnisonBendManual),
        (84, UnisonBendManual),
        (85, SusDown),
    ] {
        let events = raw_notes(&[(0.0, 0.5, pitch)]);
        assert_eq!(
            articulations(&events, &manual_on(&[0])),
            vec![expected],
            "pitch {pitch}"
        );
    }
}

#[test]
fn a_chord_with_a_note_outside_the_range_is_not_bent() {
    let events = raw_notes(&[(0.0, 0.5, 55), (0.0, 0.5, 67)]);
    assert_eq!(
        articulations(&events, &manual_on(&[0])),
        vec![SusDown, SusDown]
    );
    let events = raw_notes(&[(0.0, 0.5, 64), (0.0, 0.5, 67)]);
    assert_eq!(
        articulations(&events, &manual_on(&[0])),
        vec![UnisonBendManual, UnisonBendManual]
    );
}

#[test]
fn a_column_overlapped_by_another_column_is_not_bent() {
    // 0 列目（0〜1 秒）の間に 1 列目が鳴り始める。1 列目も 0 列目と重なる。
    let overlapped = raw_notes(&[(0.0, 1.0, 67), (0.5, 1.5, 64)]);
    assert_eq!(
        articulations(&overlapped, &manual_on(&[0, 1])),
        vec![SusDown, SusDown]
    );
    assert!(bends(&convert(&overlapped, &manual_on(&[0, 1]))).is_empty());
    // 前の列の note off と同じ時刻に始まるのは重なりではない。
    let adjacent = raw_notes(&[(0.0, 0.5, 67), (0.5, 1.0, 64)]);
    assert_eq!(
        articulations(&adjacent, &manual_on(&[0, 1])),
        vec![UnisonBendManual, UnisonBendManual]
    );
}

#[test]
fn the_bend_rises_holds_and_returns_to_center_at_the_note_off() {
    // o5 l2 g: 0〜1 秒。上げるのは 0.35 秒、戻すのは 0.13 秒。
    let out = convert(&raw("o5 l2 g"), &manual_on(&[0]));
    let bends = bends(&out);
    assert_eq!(bends.first(), Some(&(0.0, CENTER)));
    let top = bends.iter().position(|&(_, v)| v == TOP).unwrap();
    assert!((bends[top].0 - UNISON_BEND_RISE_SECONDS).abs() < 1e-9);
    assert!(bends[..=top]
        .windows(2)
        .all(|w| w[0].1 < w[1].1 && w[0].0 < w[1].0));
    let fall_start = bends.iter().rposition(|&(_, v)| v == TOP).unwrap();
    assert!((bends[fall_start].0 - (1.0 - UNISON_BEND_FALL_SECONDS)).abs() < 1e-9);
    assert!(bends[fall_start..bends.len() - 1]
        .windows(2)
        .all(|w| w[0].1 > w[1].1));
    // 列の note off で中央、演奏の終わりでもう 1 回中央。
    let n = bends.len();
    assert_eq!(bends[n - 2], (1.0, CENTER));
    assert_eq!(bends[n - 1], (1.0, CENTER));
    // 刻みはサンプル MID と同じ程度。
    assert!(bends[1].0 - bends[0].0 <= UNISON_BEND_STEP_SECONDS + 1e-9);
    // 中央へ戻すイベントは同じ時刻の note off より後に並ぶ。
    let off = out
        .iter()
        .position(|e| e.seconds == 1.0 && e.message[1] == 67 && e.message[0] & 0xF0 != 0xE0)
        .unwrap();
    let last_bend = out
        .iter()
        .rposition(|e| e.message[0] & 0xF0 == 0xE0)
        .unwrap();
    assert!(off < last_bend);
}

#[test]
fn a_short_note_shrinks_the_rise_and_the_fall() {
    // o5 l16 g: 0.125 秒。上げるのは 60%、戻すのは 20%。
    let bends = bends(&convert(&raw("o5 l16 g"), &manual_on(&[0])));
    let top = bends.iter().position(|&(_, v)| v == TOP).unwrap();
    assert!((bends[top].0 - 0.125 * 0.6).abs() < 1e-9);
    let fall_start = bends.iter().rposition(|&(_, v)| v == TOP).unwrap();
    assert!((bends[fall_start].0 - 0.125 * 0.8).abs() < 1e-9);
    assert!(bends
        .iter()
        .all(|&(t, _)| (0.0..=0.125 + 1e-9).contains(&t)));
}

#[test]
fn no_bend_without_the_rule_or_outside_the_range() {
    assert!(bends(&convert(&raw("o5 l2 g"), &RuleTable::default())).is_empty());
    assert!(bends(&convert(&raw("o3 l2 g"), &manual_on(&[0]))).is_empty());
    // 自動版は pitch bend を送らない。
    let mut auto = RuleTable::default();
    auto.toggle(0, Rule::UnisonBendAuto);
    assert!(bends(&convert(&raw("o5 l2 g"), &auto)).is_empty());
}

#[test]
fn the_bend_is_sent_in_single_note_mode_and_with_humanize() {
    let events = raw("o5 l4 e g2 a");
    let rules = manual_on(&[1]);
    let notes = notes_from_events(&events);
    let articulated = articulate(&notes, &rules);
    let single = bends(&column_events(
        &notes,
        &articulated,
        &rules,
        1,
        Take::Converted,
    ));
    assert_eq!(single.first(), Some(&(0.0, CENTER)));
    assert!(single.iter().any(|&(_, v)| v == TOP));
    assert_eq!(single.last(), Some(&(1.0, CENTER)));

    let mut humanized = rules.clone();
    humanized.toggle_row(RowRule::Humanize);
    let out = bends(&convert(&events, &humanized));
    assert!(out.iter().any(|&(_, v)| v == TOP));
    assert_eq!(out.last().map(|&(_, v)| v), Some(CENTER));
}

#[test]
fn the_rule_round_trips_through_json() {
    let rules = manual_on(&[2]);
    let json = rules.to_json();
    assert!(json.contains("\"unison_bend_manual\""), "{json}");
    assert_eq!(RuleTable::from_json(&json).unwrap(), rules);
}
