use super::*;
use crate::{column_events, convert, Articulated, RowRule, Take};

const MUTE_LENGTH: u8 = 22;
const TENSION: u8 = 46;

fn raw(mml: &str) -> Vec<TimedMidiEvent> {
    cmrt_chord::timed_performance(mml).unwrap().events
}

/// `convert` の CC のうち、パラメータのものだけ。
fn param_ccs(events: &[TimedMidiEvent]) -> Vec<(f64, [u8; 3])> {
    events
        .iter()
        .filter(|e| e.message[0] & 0xF0 == 0xB0 && param_of(e.message[1]).is_some())
        .map(|e| (e.seconds, e.message))
        .collect()
}

#[test]
fn values_equal_to_the_sfz_default_are_not_sent() {
    let mut rules = RuleTable::default();
    assert_eq!(rules.param(MUTE_LENGTH), 51);
    assert!(param_ccs(&convert(&raw("o3 l4 e g"), &rules)).is_empty());

    // 動かして戻すと、値も JSON も既定のまま。
    assert!(rules.step_param(MUTE_LENGTH, -8));
    assert!(rules.step_param(MUTE_LENGTH, 8));
    assert_eq!(rules, RuleTable::default());
    assert!(param_ccs(&convert(&raw("o3 l4 e g"), &rules)).is_empty());
}

#[test]
fn changed_values_are_sent_at_the_head_and_reset_at_the_end() {
    let mut rules = RuleTable::default();
    rules.step_param(TENSION, 127);
    for _ in 0..7 {
        rules.step_param(MUTE_LENGTH, -8);
    }
    assert_eq!(rules.param(MUTE_LENGTH), 0);
    assert_eq!(rules.param(TENSION), 127);

    // E2 → G2、l4 = 0.5 秒ずつ。CC の小さい順に、頭で送り終わりで既定へ戻す。
    let out = convert(&raw("o3 l4 e g"), &rules);
    assert_eq!(
        param_ccs(&out),
        vec![
            (0.0, [0xB0, MUTE_LENGTH, 0]),
            (0.0, [0xB0, TENSION, 127]),
            (1.0, [0xB0, MUTE_LENGTH, 51]),
            (1.0, [0xB0, TENSION, 0]),
        ]
    );
    // 頭の CC は最初の音より前、戻しは最後の note off より後。
    let first_note_on = out
        .iter()
        .position(|e| e.message[0] == 0x90 && e.message[1] == 40)
        .unwrap();
    let head = out
        .iter()
        .position(|e| e.message == [0xB0, TENSION, 127])
        .unwrap();
    assert!(head < first_note_on);
    let last_note_off = out
        .iter()
        .rposition(|e| {
            e.message[0] & 0xF0 == 0x80 || (e.message[0] & 0xF0 == 0x90 && e.message[2] == 0)
        })
        .unwrap();
    let reset = out
        .iter()
        .position(|e| e.message == [0xB0, TENSION, 0])
        .unwrap();
    assert!(reset > last_note_off);
}

#[test]
fn steps_stay_within_0_to_127() {
    let mut rules = RuleTable::default();
    // CC46 の既定は 0。下へは動かない。
    assert!(!rules.step_param(TENSION, -8));
    assert_eq!(rules.param(TENSION), 0);
    for _ in 0..20 {
        rules.step_param(TENSION, 8);
    }
    assert_eq!(rules.param(TENSION), 127);
    assert!(!rules.step_param(TENSION, 8));
    // 表に無い CC は変えない。
    assert!(!rules.step_param(20, 8));
    assert_eq!(
        rules.changed_params().collect::<Vec<_>>(),
        vec![(TENSION, 127)]
    );
}

#[test]
fn params_round_trip_through_json() {
    let mut rules = RuleTable::default();
    rules.toggle_row(RowRule::Humanize);
    rules.step_param(MUTE_LENGTH, -51);
    rules.step_param(112, 127);
    let json = rules.to_json();
    assert_eq!(
        json,
        r#"{"columns":{},"rows":["humanize"],"params":{"22":0,"112":127}}"#
    );
    assert_eq!(RuleTable::from_json(&json).unwrap(), rules);

    // params の無い JSON も読める。表に無い CC・既定と同じ値・127 を超える値は捨てる。
    assert_eq!(
        RuleTable::from_json(r#"{"rows":["humanize"]}"#).unwrap(),
        RuleTable::from_json(r#"{"rows":["humanize"],"params":{"20":5,"22":51,"46":200}}"#)
            .unwrap()
    );
    assert!(RuleTable::from_json(r#"{"params":{"22":"x"}}"#).is_err());
}

#[test]
fn params_survive_rewriting_the_mml() {
    let mut rules = RuleTable::default();
    rules.toggle(0, crate::Rule::PalmMute);
    rules.step_param(MUTE_LENGTH, -8);
    let kept = rules.without_column_rules();
    assert!(kept.is_empty());
    assert_eq!(kept.param(MUTE_LENGTH), 43);

    let anchor = crate::ColumnRuleAnchor::new("o3 e g", &Default::default(), &rules);
    assert_eq!(anchor.rules.param(MUTE_LENGTH), 51);
    let notes = crate::notes_from_events(&raw("o3 e g a"));
    let remapped = anchor.rules_for(&notes, &rules);
    assert!(remapped.is_on(0, crate::Rule::PalmMute));
    assert_eq!(remapped.param(MUTE_LENGTH), 43);
}

#[test]
fn one_note_and_humanized_takes_also_send_the_params() {
    let mut rules = RuleTable::default();
    rules.step_param(TENSION, 64);
    let events = raw("o3 l4 e g");
    let notes = crate::notes_from_events(&events);
    let articulated: Vec<Articulated> = crate::articulate(&notes, &rules);
    // 1 音モードの 2 列目は 0 秒から 0.5 秒。
    assert_eq!(
        param_ccs(&column_events(
            &notes,
            &articulated,
            &rules,
            1,
            Take::Converted
        )),
        vec![(0.0, [0xB0, TENSION, 64]), (0.5, [0xB0, TENSION, 0])]
    );
    assert!(param_ccs(&column_events(&notes, &articulated, &rules, 1, Take::Plain)).is_empty());

    rules.toggle_row(RowRule::Humanize);
    let ccs = param_ccs(&convert(&events, &rules));
    assert_eq!(ccs.len(), 2);
    assert_eq!(ccs[0].1, [0xB0, TENSION, 64]);
    assert_eq!(ccs[1].1, [0xB0, TENSION, 0]);
}
