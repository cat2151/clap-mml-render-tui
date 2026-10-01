//! 演奏の頭の列 CC（[`add_column_cc_defaults`]）: 頭の時刻に、[`COLUMN_CCS`] の CC がちょうど 1 つずつ在る。

use super::*;
use crate::release::RELEASE_SHAPE_RANGE;
use crate::{convert, RowRule};

/// 頭が 0 秒でない行（休符始まり）と、頭が和音の行。
const MMLS: [&str; 2] = ["r8 o3 l8 e g a", "r8 o3 l8 'eg' a b"];

fn raw(mml: &str) -> Vec<TimedMidiEvent> {
    cmrt_chord::timed_performance(mml).unwrap().events
}

/// 演奏のいちばん早い note on の時刻（KS は 28 未満の音高なので除く）。
fn first_note_on(events: &[TimedMidiEvent]) -> f64 {
    events
        .iter()
        .filter(|e| e.message[0] & 0xF0 == 0x90 && e.message[1] >= 28 && e.message[2] > 0)
        .map(|e| e.seconds)
        .fold(f64::INFINITY, f64::min)
}

/// 頭の時刻にある `controller` の値。
fn head_values(events: &[TimedMidiEvent], controller: u8) -> Vec<u8> {
    let first = first_note_on(events);
    values_of(events, controller)
        .into_iter()
        .filter(|(at, _)| *at == first)
        .map(|(_, value)| value)
        .collect()
}

/// 1 列目（列 0）に `rule` を ON にし、`unaffected` なら 1 列目をピンチハーモニクスにして効かなくする。
fn table(rule: Option<Rule>, unaffected: bool, rows: &[RowRule]) -> RuleTable {
    let mut columns = Vec::new();
    if let Some(rule) = rule {
        columns.push((0, rule));
    }
    if unaffected {
        columns.push((0, Rule::PinchHarmonic));
    }
    rules_on(&columns, rows)
}

#[test]
fn every_column_cc_appears_exactly_once_at_the_head_with_the_right_value() {
    let rules = [
        Rule::Vibrato,
        Rule::LongExtra,
        Rule::PowerChord,
        Rule::PositionRelease,
        Rule::AutoSlideOut,
    ];
    let mut cases = 0;
    for mml in MMLS {
        let notes = notes_from_events(&raw(mml));
        for release in [false, true] {
            for humanize in [false, true] {
                let rows: Vec<RowRule> = [
                    release.then_some(RowRule::HumanizeRelease),
                    humanize.then_some(RowRule::Humanize),
                ]
                .into_iter()
                .flatten()
                .collect();
                for on in std::iter::once(None).chain(rules.map(Some)) {
                    for unaffected in [false, true] {
                        // ビブラートはどの奏法の音にも効くので、効かない 1 列目は作れない。
                        if unaffected && matches!(on, None | Some(Rule::Vibrato)) {
                            continue;
                        }
                        let table = table(on, unaffected, &rows);
                        if unaffected {
                            let articulated = articulate(&notes, &table);
                            assert_eq!(articulated[0].articulation, Articulation::PinchHarmonic);
                            assert!(!control_rule_affects(
                                on.unwrap(),
                                articulated[0].articulation,
                                notes[0].pitch
                            ));
                        }
                        let out = convert(&raw(mml), &table);
                        let label = format!(
                            "{mml} release={release} humanize={humanize} on={on:?} unaffected={unaffected}"
                        );
                        if !humanize {
                            assert_eq!(first_note_on(&out), 0.25, "{label}");
                        }
                        for cc in &COLUMN_CCS {
                            let (controller, default) = (cc.controller, cc.default);
                            let head = head_values(&out, controller);
                            assert_eq!(head.len(), 1, "CC{controller} {label}: {head:?}");
                            let ruled = cc
                                .rules
                                .iter()
                                .find(|&&(rule, _)| on == Some(rule) && !unaffected);
                            if let Some(&(_, value)) = ruled {
                                assert_eq!(head[0], value, "CC{controller} {label}");
                            } else if release && controller == RELEASE_SHAPE_CC {
                                assert!(
                                    RELEASE_SHAPE_RANGE.contains(&head[0]),
                                    "CC{controller} {label}: {}",
                                    head[0]
                                );
                            } else {
                                assert_eq!(head[0], default, "CC{controller} {label}");
                            }
                        }
                        cases += 1;
                    }
                }
            }
        }
    }
    // MML 2 × リリース 2 × 汚し 2 × (全 OFF 1 + ルール 5 + 効かない 1 列目 4)。
    assert_eq!(cases, 2 * 2 * 2 * 10);
}

#[test]
fn single_note_columns_also_get_exactly_one_head_cc_each() {
    let raw = raw("o3 l8 e g a");
    let notes = notes_from_events(&raw);
    for cc in &COLUMN_CCS {
        let controller = cc.controller;
        for &(rule, value) in cc.rules {
            for on in [false, true] {
                let columns: &[(usize, Rule)] = if on { &[(1, rule)] } else { &[] };
                let table = rules_on(columns, &[]);
                let articulated = articulate(&notes, &table);
                let out =
                    crate::column_events(&notes, &articulated, &table, 1, crate::Take::Converted);
                let head = head_values(&out, controller);
                let expected = if on { value } else { cc.default };
                assert_eq!(head, vec![expected], "CC{controller} {rule:?} on={on}");
            }
        }
    }
}

#[test]
fn the_head_is_the_earliest_note_on_even_when_a_later_column_sounds_first() {
    // convert の列は時刻順で、汚しは隣の列を追い越さない（NEIGHBOR_GAP_FRACTION < 0.5）ので、
    // 2 列目が先に鳴る行は作れない。頭を決める関数へ直接渡して確かめる。
    let note = |column, on_seconds| crate::Note {
        column,
        on_seconds,
        off_seconds: on_seconds + 0.25,
        pitch: 52,
        velocity: 100,
        channel: 0,
    };
    let notes = [note(0, 0.30), note(1, 0.20)];
    let mut out = vec![control_change(0.30, 0, VIBRATO_DEPTH_CC, VIBRATO_DEPTH)];
    add_column_cc_defaults(&mut out, &notes);
    for &ColumnCc {
        controller,
        default,
        ..
    } in &COLUMN_CCS
    {
        assert_eq!(
            values_of(&out, controller)
                .into_iter()
                .filter(|(at, _)| *at == 0.20)
                .collect::<Vec<_>>(),
            vec![(0.20, default)],
            "CC{controller}"
        );
    }
    // 頭でない時刻の CC は、頭に既定値を足すかどうかに関わらない。
    assert_eq!(out.len(), COLUMN_CCS.len() + 1);
    let mut empty = Vec::new();
    add_column_cc_defaults(&mut empty, &[]);
    assert!(empty.is_empty());
}
