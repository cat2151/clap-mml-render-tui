use super::*;
use crate::{articulate, notes_from_events, Rule, RuleTable};

fn events_for(mml: &str, rules: &RuleTable) -> Vec<TimedMidiEvent> {
    let notes = notes_from_events(&cmrt_chord::timed_performance(mml).unwrap().events);
    let articulations: Vec<Articulation> = articulate(&notes, rules)
        .iter()
        .map(|a| a.articulation)
        .collect();
    control_events(&notes, &articulations, rules)
}

fn slide_on(columns: &[usize]) -> RuleTable {
    let mut rules = RuleTable::default();
    for &column in columns {
        rules.toggle(column, Rule::Slide);
    }
    rules
}

#[test]
fn a_slide_sends_its_width_then_resets_at_the_end() {
    // E2 → G2 は +3。l8 = 0.25 秒。
    let out = events_for("o3 l8 e g", &slide_on(&[1]));
    assert_eq!(
        out,
        vec![
            control_change(0.25, 0, SLIDE_WIDTH_CC, 40),
            control_change(0.5, 0, SLIDE_WIDTH_CC, SLIDE_WIDTH_CC_DEFAULT),
        ]
    );
}

#[test]
fn width_values_are_the_middle_of_each_sfz_range() {
    assert_eq!(slide_width_value(1), 8);
    assert_eq!(slide_width_value(2), 24);
    assert_eq!(slide_width_value(SLIDE_MAX_SEMITONES), 104);
}

#[test]
fn no_slide_no_control_change() {
    assert!(events_for("o3 l8 e g a", &RuleTable::default()).is_empty());
    // 範囲外（+11）で滑らなければ戻しも送らない。
    assert!(events_for("o3 l8 c b", &slide_on(&[1])).is_empty());
    let mut choke = RuleTable::default();
    choke.toggle(1, Rule::Choke);
    assert!(events_for("o3 l8 e g", &choke).is_empty());
}

fn vibrato_on(columns: &[usize]) -> RuleTable {
    let mut rules = RuleTable::default();
    for &column in columns {
        rules.toggle(column, Rule::Vibrato);
    }
    rules
}

fn vibrato(seconds: f64, value: u8) -> TimedMidiEvent {
    control_change(seconds, 0, VIBRATO_DEPTH_CC, value)
}

#[test]
fn a_vibrato_column_sends_depth_at_on_and_zero_at_off_then_resets() {
    let out = events_for("o3 l8 e g a", &vibrato_on(&[1]));
    assert_eq!(
        out,
        vec![
            vibrato(0.25, VIBRATO_DEPTH),
            vibrato(0.5, 0),
            vibrato(0.75, 0)
        ]
    );
}

#[test]
fn a_vibrato_chord_sends_once_per_column() {
    let out = events_for("o3 l8 e 'gb<d' a", &vibrato_on(&[1]));
    assert_eq!(
        out,
        vec![
            vibrato(0.25, VIBRATO_DEPTH),
            vibrato(0.5, 0),
            vibrato(0.75, 0)
        ]
    );
}

#[test]
fn back_to_back_vibrato_columns_reset_before_the_next_depth() {
    let out = events_for("o3 l8 e g a", &vibrato_on(&[1, 2]));
    assert_eq!(
        out,
        vec![
            vibrato(0.25, VIBRATO_DEPTH),
            vibrato(0.5, 0),
            vibrato(0.5, VIBRATO_DEPTH),
            vibrato(0.75, 0),
            vibrato(0.75, 0)
        ]
    );
}

#[test]
fn no_vibrato_no_cc20() {
    let out = events_for("o3 l8 e g a e", &slide_on(&[1, 2, 3]));
    assert!(out.iter().all(|e| e.message[1] != VIBRATO_DEPTH_CC));
}

fn slide_in(seconds: f64, value: u8) -> TimedMidiEvent {
    control_change(seconds, 0, SLIDE_IN_WIDTH_CC, value)
}

fn slide_in_on(columns: &[usize]) -> RuleTable {
    let mut rules = RuleTable::default();
    for &column in columns {
        rules.toggle(column, Rule::SlideIn);
    }
    rules
}

#[test]
fn a_slide_in_sends_the_middle_of_its_width_band_then_resets_to_82() {
    // E2 → G2 は幅 3 = 40。
    assert_eq!(
        events_for("o3 l8 e g", &slide_in_on(&[1])),
        vec![slide_in(0.25, 40), slide_in(0.5, SLIDE_IN_WIDTH_CC_DEFAULT)]
    );
    // 先頭の列は幅 1 = 8、+11 は幅 7 = 104。
    assert_eq!(
        events_for("o3 l8 c b", &slide_in_on(&[0, 1])),
        vec![
            slide_in(0.0, 8),
            slide_in(0.25, 104),
            slide_in(0.5, SLIDE_IN_WIDTH_CC_DEFAULT)
        ]
    );
    assert_eq!(SLIDE_IN_WIDTH_CC_DEFAULT, 82);
}

#[test]
fn a_slide_in_below_the_lowest_pitch_of_its_width_sends_nothing() {
    // C1 → G1（31）は幅 7 で下端 32 の外。
    assert!(events_for("o2 l8 c g", &slide_in_on(&[1])).is_empty());
}

fn rules_on(rules: &[(usize, Rule)], rows: &[crate::RowRule]) -> RuleTable {
    let mut table = RuleTable::default();
    for &(column, rule) in rules {
        table.toggle(column, rule);
    }
    for &row in rows {
        table.toggle_row(row);
    }
    table
}

fn values_of(events: &[TimedMidiEvent], controller: u8) -> Vec<(f64, u8)> {
    events
        .iter()
        .filter(|e| e.message[0] & 0xF0 == 0xB0 && e.message[1] == controller)
        .map(|e| (e.seconds, e.message[2]))
        .collect()
}

#[test]
fn long_extra_and_power_chord_are_sent_like_vibrato_on_sus_down() {
    for controller in [LONG_EXTRA_CC, POWER_CHORD_CC] {
        let rule = if controller == LONG_EXTRA_CC {
            Rule::LongExtra
        } else {
            Rule::PowerChord
        };
        let out = events_for("o3 l8 e g a", &rules_on(&[(1, rule)], &[]));
        assert_eq!(
            out,
            vec![
                control_change(0.25, 0, controller, 127),
                control_change(0.5, 0, controller, 0),
                control_change(0.75, 0, controller, 0),
            ],
            "CC{controller}"
        );
    }
    assert!(!Rule::LongExtra.is_exclusive());
    assert!(!Rule::PowerChord.is_exclusive());
    assert!(!Rule::PositionRelease.is_exclusive());
}

#[test]
fn long_extra_reaches_mute_down_but_power_chord_does_not() {
    // パームミュートの列は Mute_Down。CC23 は Mute_EX を選ぶが、CC32 の層は Sus にしか無い。
    let muted = [(1, Rule::PalmMute)];
    let long = events_for(
        "o3 l8 e g a",
        &rules_on(&[muted[0], (1, Rule::LongExtra)], &[]),
    );
    assert_eq!(
        values_of(&long, LONG_EXTRA_CC),
        vec![(0.25, 127), (0.5, 0), (0.75, 0)]
    );
    let power = events_for(
        "o3 l8 e g a",
        &rules_on(&[muted[0], (1, Rule::PowerChord)], &[]),
    );
    assert!(values_of(&power, POWER_CHORD_CC).is_empty());
}

#[test]
fn long_extra_skips_an_up_stroke_but_power_chord_keeps_it() {
    // エコノミーピッキングで E2 → G2 は同じ弦のオルタネイト（2 音目がアップ）。
    let eco = [crate::RowRule::EconomyPicking];
    let notes = notes_from_events(&cmrt_chord::timed_performance("o3 l8 e g a").unwrap().events);
    let rules = rules_on(&[(1, Rule::LongExtra)], &eco);
    assert_eq!(
        articulate(&notes, &rules)[1].articulation,
        Articulation::SusUp
    );
    assert!(events_for("o3 l8 e g a", &rules).is_empty());
    let power = events_for("o3 l8 e g a", &rules_on(&[(1, Rule::PowerChord)], &eco));
    assert_eq!(values_of(&power, POWER_CHORD_CC).len(), 3);
}

#[test]
fn position_release_sends_72_then_back_to_13_only_inside_its_pitches() {
    let out = events_for("o3 l8 e g a", &rules_on(&[(1, Rule::PositionRelease)], &[]));
    assert_eq!(
        values_of(&out, 24),
        vec![(0.25, POSITION_RELEASE_VALUE), (0.5, 13), (0.75, 13)]
    );
    // E6（88）は release5 の音域（30〜76）の外。パームミュートの CC24 はアップストロークの選択なので送らない。
    assert!(events_for("o7 l8 e e", &rules_on(&[(1, Rule::PositionRelease)], &[])).is_empty());
    let muted = rules_on(&[(1, Rule::PalmMute), (1, Rule::PositionRelease)], &[]);
    assert!(events_for("o3 l8 e g a", &muted).is_empty());
}

#[test]
fn position_release_wins_over_humanize_release_in_its_column_only() {
    let raw = cmrt_chord::timed_performance("o3 l8 e g a e")
        .unwrap()
        .events;
    let release = [crate::RowRule::HumanizeRelease];
    let without = crate::convert(&raw, &rules_on(&[], &release));
    let with = crate::convert(&raw, &rules_on(&[(1, Rule::PositionRelease)], &release));
    // 同時刻の CC24 は後の値が効く。列ごとの頭の、最後の CC24 を比べる。
    let last_at = |events: &[TimedMidiEvent], seconds: f64| {
        values_of(events, 24)
            .into_iter()
            .rfind(|(at, _)| *at == seconds)
            .map(|(_, value)| value)
    };
    for (column, seconds) in [0.0, 0.25, 0.5, 0.75].into_iter().enumerate() {
        let expected = if column == 1 {
            Some(POSITION_RELEASE_VALUE)
        } else {
            last_at(&without, seconds)
        };
        assert_eq!(last_at(&with, seconds), expected, "列 {column}");
    }
    // 列の終わりでは戻さない（次の列の汚し（リリース）の値を消さない）。演奏の終わりは既定へ戻る。
    assert_eq!(values_of(&with, 24).last(), Some(&(1.0, 13)));
}

#[test]
fn humanize_puts_accents_on_sus_ex_and_the_rest_on_sus_lt() {
    // Sus_Down + CC23 は velocity 111 以下で Sus_LT、112 以上で Sus_EX。
    let raw = cmrt_chord::timed_performance("o3 l8 e f+ g a b a g f+")
        .unwrap()
        .events;
    let out = crate::convert(&raw, &rules_on(&[], &[crate::RowRule::Humanize]));
    let notes = notes_from_events(&raw);
    let accents = crate::articulate(&notes, &rules_on(&[], &[crate::RowRule::Humanize]));
    let velocities: Vec<u8> = out
        .iter()
        .filter(|e| e.message[0] & 0xF0 == 0x90 && e.message[1] >= 28 && e.message[2] > 0)
        .map(|e| e.message[2])
        .collect();
    assert_eq!(velocities.len(), notes.len());
    for (velocity, articulated) in velocities.iter().zip(&accents) {
        assert_eq!(*velocity >= 112, articulated.accent, "{velocities:?}");
    }
    assert!(accents.iter().any(|a| a.accent) && accents.iter().any(|a| !a.accent));
}
