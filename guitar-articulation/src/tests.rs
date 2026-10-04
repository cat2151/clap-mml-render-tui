use std::collections::BTreeSet;

use super::*;

fn event(seconds: f64, message: [u8; 3]) -> TimedMidiEvent {
    TimedMidiEvent { seconds, message }
}

/// E2 → F#2 → G2（各 0.5 秒）のraw。
fn flat() -> Vec<TimedMidiEvent> {
    vec![
        event(0.0, [0x90, 40, 100]),
        event(0.5, [0x80, 40, 0]),
        event(0.5, [0x90, 42, 100]),
        event(1.0, [0x80, 42, 0]),
        event(1.0, [0x90, 43, 100]),
        event(1.5, [0x80, 43, 0]),
    ]
}

/// 列で送る CC（CC20 / CC23 / CC32 / CC24）の既定値。演奏のいちばん早い note on の時刻に送る。
fn column_cc_defaults(seconds: f64) -> Vec<TimedMidiEvent> {
    [[0xB0, 20, 0], [0xB0, 23, 0], [0xB0, 32, 0], [0xB0, 24, 13]]
        .into_iter()
        .map(|message| event(seconds, message))
        .collect()
}

fn is_keyswitch(e: &TimedMidiEvent) -> bool {
    e.message[1] < 30 && matches!(e.message[0] & 0xF0, 0x80 | 0x90)
}

#[test]
fn empty_rules_add_only_the_head_sus_down() {
    let out = convert(&flat(), &RuleTable::default());

    let mut expected = flat();
    expected.insert(0, event(0.0, [0x90, 17, KEYSWITCH_VELOCITY]));
    expected.insert(2, event(0.5, [0x80, 17, 0]));
    expected.splice(0..0, column_cc_defaults(0.0));
    assert_eq!(out, expected);
}

#[test]
fn keyswitch_note_on_comes_before_the_note_on_at_the_same_time() {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::HammerPull);
    let out = convert(&flat(), &rules);

    let at_half: Vec<[u8; 3]> = out
        .iter()
        .filter(|e| e.seconds == 0.5)
        .map(|e| e.message)
        .collect();
    let ks = at_half.iter().position(|m| *m == [0x90, 26, 127]).unwrap();
    let played = at_half.iter().position(|m| *m == [0x90, 42, 100]).unwrap();
    assert!(ks < played, "{at_half:?}");
}

#[test]
fn rules_do_not_move_the_played_notes() {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::HammerPull);
    rules.toggle(2, Rule::HammerPull);
    let out = convert(&flat(), &rules);

    let played: Vec<TimedMidiEvent> = out
        .iter()
        .copied()
        .filter(|e| !is_keyswitch(e) && e.message[0] & 0xF0 != 0xB0)
        .collect();
    assert_eq!(played, flat());
    let keyswitch_on: Vec<(f64, u8)> = out
        .iter()
        .filter(|e| is_keyswitch(e) && e.message[0] & 0xF0 == 0x90)
        .map(|e| (e.seconds, e.message[1]))
        .collect();
    // 2・3 音目は続けて Hammer-On なので 26 は 1 回。最後が 26 なので末尾で 17 に戻す。
    assert_eq!(keyswitch_on, vec![(0.0, 17), (0.5, 26), (1.5, 17)]);
}

#[test]
fn empty_input_stays_empty() {
    assert!(convert(&[], &RuleTable::default()).is_empty());
}

fn from_mml(mml: &str) -> Vec<TimedMidiEvent> {
    cmrt_chord::timed_performance(mml).unwrap().events
}

#[test]
fn real_mml_melody_gets_hammer_on_and_pull_off() {
    let events = from_mml("o3 l8 e f+ e");
    let notes = notes_from_events(&events);
    let columns: Vec<(usize, u8)> = notes.iter().map(|n| (n.column, n.pitch)).collect();
    assert_eq!(columns, vec![(0, 40), (1, 42), (2, 40)]);

    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::HammerPull);
    rules.toggle(2, Rule::HammerPull);
    let keyswitch_on: Vec<u8> = convert(&events, &rules)
        .iter()
        .filter(|e| is_keyswitch(e) && e.message[0] & 0xF0 == 0x90)
        .map(|e| e.message[1])
        .collect();
    assert_eq!(keyswitch_on, vec![17, 26, 25, 17]);
}

#[test]
fn real_mml_chord_is_one_column() {
    let notes = notes_from_events(&from_mml("o3 l8 'egb' e"));
    let columns: Vec<usize> = notes.iter().map(|n| n.column).collect();
    assert_eq!(columns, vec![0, 0, 0, 1]);
}

#[test]
fn economy_picking_adds_sus_up_and_lowers_all_but_the_accents() {
    // E2 → F#2 → G2 → F#2。アクセントは頭の E2 と頂点の G2。
    let mut events = flat();
    events.push(event(1.5, [0x90, 42, 100]));
    events.push(event(2.0, [0x80, 42, 0]));
    let mut rules = RuleTable::default();
    rules.toggle_row(RowRule::EconomyPicking);
    let out = convert(&events, &rules);

    assert!(
        out.iter()
            .any(|e| e.message == [0x90, 18, KEYSWITCH_VELOCITY]),
        "2 音目はアップ: {out:?}"
    );
    let played: Vec<[u8; 3]> = out
        .iter()
        .filter(|e| !is_keyswitch(e) && e.message[0] == 0x90)
        .map(|e| e.message)
        .collect();
    assert_eq!(
        played,
        vec![
            [0x90, 40, 100],
            [0x90, 42, 75],
            [0x90, 43, 100],
            [0x90, 42, 75]
        ]
    );
}

#[test]
fn the_rule_table_round_trips_through_json() {
    let mut rules = RuleTable::default();
    rules.toggle(3, Rule::HammerPull);
    rules.toggle_row(RowRule::EconomyPicking);

    let json = rules.to_json();
    assert_eq!(
        json,
        r#"{"columns":{"3":["hammer_pull"]},"rows":["economy_picking"]}"#
    );
    assert_eq!(RuleTable::from_json(&json).unwrap(), rules);
    assert_eq!(RuleTable::from_json("{}").unwrap(), RuleTable::default());
    assert!(RuleTable::from_json(r#"{"rows":["nope"]}"#).is_err());
}

#[test]
fn economy_picking_and_auto_hammer_pull_turn_each_other_off() {
    let mut rules = RuleTable::default();
    rules.toggle_row(RowRule::Humanize);
    rules.toggle_row(RowRule::EconomyPicking);
    rules.toggle_row(RowRule::AutoHammerPull);
    assert!(rules.is_row_on(RowRule::AutoHammerPull));
    assert!(!rules.is_row_on(RowRule::EconomyPicking));
    assert!(rules.is_row_on(RowRule::Humanize));

    rules.toggle_row(RowRule::EconomyPicking);
    assert!(rules.is_row_on(RowRule::EconomyPicking));
    assert!(!rules.is_row_on(RowRule::AutoHammerPull));
    assert!(rules.is_row_on(RowRule::Humanize));
}

#[test]
fn json_with_both_economy_picking_and_auto_hammer_pull_keeps_only_economy_picking() {
    let rules =
        RuleTable::from_json(r#"{"rows":["auto_hammer_pull","economy_picking","humanize"]}"#)
            .unwrap();
    let mut expected = RuleTable::default();
    expected.toggle_row(RowRule::EconomyPicking);
    expected.toggle_row(RowRule::Humanize);
    assert_eq!(rules, expected);
    assert!(RuleTable::from_json(r#"{"rows":[],"extra":1}"#).is_err());
}

#[test]
fn every_articulation_round_trips_through_its_keyswitch() {
    for articulation in Articulation::ALL {
        assert_eq!(
            Articulation::from_keyswitch(articulation.keyswitch()),
            Some(articulation)
        );
    }
    let keys: BTreeSet<u8> = Articulation::ALL.iter().map(|a| a.keyswitch()).collect();
    assert_eq!(keys.len(), Articulation::ALL.len(), "KS 番号が重複している");
}

#[test]
fn an_exclusive_rule_turns_off_the_other_exclusive_rules_in_the_column() {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::HammerPull);
    rules.toggle(2, Rule::HammerPull);
    rules.toggle(1, Rule::PalmMute);
    assert!(!rules.is_on(1, Rule::HammerPull));
    assert!(rules.is_on(1, Rule::PalmMute));
    assert!(rules.is_on(2, Rule::HammerPull), "他の列はそのまま");

    rules.toggle(1, Rule::PinchHarmonic);
    assert!(!rules.is_on(1, Rule::PalmMute));
    assert!(rules.is_on(1, Rule::PinchHarmonic));

    rules.toggle(1, Rule::PinchHarmonic);
    assert!(
        !rules.is_on(1, Rule::PinchHarmonic),
        "OFF にしても前のルールは戻らない"
    );
    assert!(!rules.is_on(1, Rule::HammerPull));
}

#[test]
fn palm_mute_and_pinch_harmonic_add_their_keyswitches() {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::PalmMute);
    rules.toggle(2, Rule::PinchHarmonic);
    let keyswitch_on: Vec<(f64, u8)> = convert(&from_mml("o3 l8 e f+ g a"), &rules)
        .iter()
        .filter(|e| is_keyswitch(e) && e.message[0] & 0xF0 == 0x90)
        .map(|e| (e.seconds, e.message[1]))
        .collect();
    assert_eq!(
        keyswitch_on,
        vec![(0.0, 17), (0.25, 20), (0.5, 10), (0.75, 17)]
    );
}

/// 新しいルールが無い表で、H/P とエコノミーピッキングを重ねたときの固定値。
#[test]
fn existing_rules_keep_their_output() {
    let mut rules = RuleTable::default();
    rules.toggle(3, Rule::HammerPull);
    rules.toggle_row(RowRule::EconomyPicking);
    let out = convert(&from_mml("o3 l8 e f+ g a b a g f+ e"), &rules);

    // 38 音ぶんのイベントと、頭の列 CC の既定値 4 つ。
    assert_eq!(out.len(), 42);
    let keyswitch_on: Vec<(f64, u8)> = out
        .iter()
        .filter(|e| is_keyswitch(e) && e.message[0] & 0xF0 == 0x90)
        .map(|e| (e.seconds, e.message[1]))
        .collect();
    let velocities: Vec<u8> = out
        .iter()
        .filter(|e| !is_keyswitch(e) && e.message[0] & 0xF0 == 0x90)
        .map(|e| e.message[2])
        .collect();
    assert_eq!(
        keyswitch_on,
        vec![
            (0.0, 17),
            (0.25, 18),
            (0.5, 17),
            (0.75, 26),
            (1.0, 17),
            (1.25, 18),
            (1.5, 25),
            (1.75, 17),
            (2.0, 18),
            (2.25, 17)
        ]
    );
    assert_eq!(velocities, vec![127, 95, 95, 127, 127, 95, 127, 95, 95]);
}

#[test]
fn existing_rules_keep_their_output_for_a_chord() {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::HammerPull);
    rules.toggle(2, Rule::HammerPull);
    rules.toggle_row(RowRule::AutoHammerPull);
    let keyswitch_on: Vec<(f64, u8)> = convert(&from_mml("o3 l8 'egb' e g <c"), &rules)
        .iter()
        .filter(|e| is_keyswitch(e) && e.message[0] & 0xF0 == 0x90)
        .map(|e| (e.seconds, e.message[1]))
        .collect();
    // 自動ハンマリングのピッキングは和音の後にダウンから交互になる。
    assert_eq!(
        keyswitch_on,
        vec![(0.0, 17), (0.5, 26), (0.75, 18), (1.0, 17)]
    );
}

#[test]
fn new_rules_round_trip_through_json() {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::PalmMute);
    rules.toggle(2, Rule::PinchHarmonic);
    let json = rules.to_json();
    assert_eq!(
        json,
        r#"{"columns":{"1":["palm_mute"],"2":["pinch_harmonic"]},"rows":[]}"#
    );
    assert_eq!(RuleTable::from_json(&json).unwrap(), rules);
}

fn cc26(out: &[TimedMidiEvent]) -> Vec<(f64, u8)> {
    out.iter()
        .filter(|e| e.message[0] & 0xF0 == 0xB0 && e.message[1] == SLIDE_WIDTH_CC)
        .map(|e| (e.seconds, e.message[2]))
        .collect()
}

#[test]
fn each_slide_width_comes_before_the_note_on_at_the_same_time() {
    // E2 → G2（+3）→ A2（+2）→ E2（-5）。列 1・2・3 がスライド。
    let mut rules = RuleTable::default();
    for column in [1, 2, 3] {
        rules.toggle(column, Rule::Slide);
    }
    let out = convert(&from_mml("o3 l8 e g a e"), &rules);

    assert_eq!(
        cc26(&out),
        vec![
            (0.25, 40),
            (0.5, 24),
            (0.75, 72),
            (1.0, SLIDE_WIDTH_CC_DEFAULT)
        ]
    );
    for (seconds, _) in cc26(&out).into_iter().take(3) {
        let at: Vec<[u8; 3]> = out
            .iter()
            .filter(|e| e.seconds == seconds)
            .map(|e| e.message)
            .collect();
        let cc = at.iter().position(|m| m[0] == 0xB0).unwrap();
        let first_on = at.iter().position(|m| m[0] == 0x90 && m[2] != 0).unwrap();
        assert!(cc < first_on, "{at:?}");
    }
}

#[test]
fn slide_and_choke_add_their_keyswitches() {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::Slide);
    rules.toggle(2, Rule::Choke);
    let keyswitch_on: Vec<(f64, u8)> = convert(&from_mml("o3 l8 e g a e"), &rules)
        .iter()
        .filter(|e| {
            e.message[0] & 0xF0 == 0x90 && Articulation::from_keyswitch(e.message[1]).is_some()
        })
        .map(|e| (e.seconds, e.message[1]))
        .collect();
    // G2 は +3 の Slide_Up、A2 は +2 の Bending_WH、E2 は既定へ戻る。
    assert_eq!(
        keyswitch_on,
        vec![(0.0, 17), (0.25, 24), (0.5, 92), (0.75, 17)]
    );
}

#[test]
fn slide_and_choke_are_exclusive_with_the_other_voicing_rules() {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::HammerPull);
    rules.toggle(1, Rule::Slide);
    assert!(!rules.is_on(1, Rule::HammerPull));
    rules.toggle(1, Rule::Choke);
    assert!(!rules.is_on(1, Rule::Slide));
    assert!(rules.is_on(1, Rule::Choke));
    rules.toggle(1, Rule::PalmMute);
    assert!(!rules.is_on(1, Rule::Choke));
}

#[test]
fn slide_and_choke_round_trip_through_json() {
    let mut rules = RuleTable::default();
    rules.toggle(1, Rule::Slide);
    rules.toggle(2, Rule::Choke);
    let json = rules.to_json();
    assert_eq!(
        json,
        r#"{"columns":{"1":["slide"],"2":["choke"]},"rows":[]}"#
    );
    assert_eq!(RuleTable::from_json(&json).unwrap(), rules);
}
