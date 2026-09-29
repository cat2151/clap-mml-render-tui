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

fn is_keyswitch(e: &TimedMidiEvent) -> bool {
    e.message[1] < 30 && matches!(e.message[0] & 0xF0, 0x80 | 0x90)
}

#[test]
fn empty_rules_add_only_the_head_sus_down() {
    let out = convert(&flat(), &RuleTable::default());

    let mut expected = flat();
    expected.insert(0, event(0.0, [0x90, 17, KEYSWITCH_VELOCITY]));
    expected.insert(2, event(0.5, [0x80, 17, 0]));
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

    let played: Vec<TimedMidiEvent> = out.iter().copied().filter(|e| !is_keyswitch(e)).collect();
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
fn economy_picking_adds_sus_up_and_lowers_all_but_the_apex() {
    // E2 → F#2 → G2 → F#2。頂点は G2。
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
            [0x90, 40, 75],
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
