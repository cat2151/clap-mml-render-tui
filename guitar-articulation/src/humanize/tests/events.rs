//! 汚しを掛けた演奏のイベント列（[`humanized_events`] と、それを呼ぶ [`convert`]）。

use std::collections::BTreeSet;

use super::*;
use crate::{convert, Rule};

/// 単音 16 分・和音混じり・連打・既定の画面と同じ形。
const MMLS: [&str; 4] = [
    OCTAVE_RUN,
    "t120 o4 l16 c 'ceg' d e 'dfa' g f e d c",
    "t150 o4 l16 c c c c d d e e d d c c",
    "o3 l8 e f+ g a g f+",
];

/// 汚しの無い表と、`auto` / `eco` / 列の H/P の組み合わせ（`auto` と `eco` は排他）。
fn rule_sets() -> Vec<RuleTable> {
    let mut columns = RuleTable::default();
    columns.toggle(2, Rule::HammerPull);
    columns.toggle(3, Rule::HammerPull);
    let mut everything = rules(&[RowRule::EconomyPicking]);
    everything.toggle(2, Rule::HammerPull);
    everything.toggle(5, Rule::HammerPull);
    vec![
        RuleTable::default(),
        rules(&[RowRule::EconomyPicking]),
        rules(&[RowRule::AutoHammerPull]),
        columns,
        everything,
    ]
}

fn with_humanize(rules: &RuleTable) -> RuleTable {
    let mut rules = rules.clone();
    rules.toggle_row(RowRule::Humanize);
    rules
}

fn raw(mml: &str) -> Vec<TimedMidiEvent> {
    cmrt_chord::timed_performance(mml).unwrap().events
}

/// `rng` で汚した演奏の列（[`convert`] と同じ並べ方）と、音符・奏法・汚しの値。
fn performance(
    mml: &str,
    rules: &RuleTable,
    rng: &mut impl Rng,
) -> (
    Vec<TimedMidiEvent>,
    Vec<Note>,
    Vec<Articulated>,
    Vec<Humanized>,
) {
    let notes = notes(mml);
    let articulated = articulate(&notes, rules);
    let humanized = humanize(&notes, &articulated, rng);
    let mut out = humanized_events(&raw(mml), &notes, &articulated, &humanized, rules);
    crate::control::add_column_cc_defaults(&mut out, &shifted_notes(&notes, &humanized));
    cmrt_midi_filter::sort_for_playback(&mut out);
    (out, notes, articulated, humanized)
}

fn seeded_performance(
    mml: &str,
    rules: &RuleTable,
    seed: u64,
) -> (
    Vec<TimedMidiEvent>,
    Vec<Note>,
    Vec<Articulated>,
    Vec<Humanized>,
) {
    performance(mml, rules, &mut StdRng::seed_from_u64(seed))
}

fn keyswitch_of(event: &TimedMidiEvent) -> Option<Articulation> {
    let [status, key, velocity] = event.message;
    (status & 0xF0 == 0x90 && velocity != 0)
        .then(|| Articulation::from_keyswitch(key))
        .flatten()
}

fn is_played_on(event: &TimedMidiEvent) -> bool {
    let [status, key, velocity] = event.message;
    status & 0xF0 == 0x90 && velocity != 0 && Articulation::from_keyswitch(key).is_none()
}

fn is_played_off(event: &TimedMidiEvent) -> bool {
    event.message[0] & 0xF0 == 0x80 && Articulation::from_keyswitch(event.message[1]).is_none()
}

fn is_cc(event: &TimedMidiEvent, controller: u8) -> bool {
    event.message[0] & 0xF0 == 0xB0 && event.message[1] == controller
}

/// 列を頭から読み、KS の note on でラッチを更新しながら、各演奏音の note on の時点のラッチがその音の奏法と一致するか。
#[test]
fn every_note_sounds_with_its_own_keyswitch_after_the_shift() {
    let mut cases = 0;
    for mml in MMLS {
        for rules in rule_sets() {
            cases += 1;
            for seed in 0..20 {
                let (out, notes, articulated, humanized) = seeded_performance(mml, &rules, seed);
                let mut latched = None;
                let mut played = 0;
                for event in &out {
                    if let Some(articulation) = keyswitch_of(event) {
                        latched = Some(articulation);
                    } else if is_played_on(event) {
                        let i = (0..notes.len())
                            .find(|&i| {
                                notes[i].pitch == event.message[1]
                                    && humanized[i].on_seconds == event.seconds
                            })
                            .expect("played note comes from a humanized note");
                        assert_eq!(
                            latched,
                            Some(articulated[i].articulation),
                            "{mml} {} seed {seed} note {i}",
                            rules.to_json()
                        );
                        played += 1;
                    }
                }
                assert_eq!(played, notes.len());
            }
        }
    }
    assert!(cases >= 8);
}

#[test]
fn the_rule_keeps_the_note_count_and_spreads_the_velocities() {
    for mml in MMLS {
        for rules in rule_sets() {
            let off = convert(&raw(mml), &rules);
            let on = convert(&raw(mml), &with_humanize(&rules));
            let count = |events: &[TimedMidiEvent], pick: fn(&TimedMidiEvent) -> bool| {
                events.iter().filter(|e| pick(e)).count()
            };
            assert_eq!(count(&on, is_played_on), count(&off, is_played_on));
            assert_eq!(count(&on, is_played_off), count(&off, is_played_off));
            let velocities = |events: &[TimedMidiEvent]| {
                events
                    .iter()
                    .filter(|e| is_played_on(e))
                    .map(|e| e.message[2])
                    .collect::<BTreeSet<_>>()
                    .len()
            };
            assert!(velocities(&on) > velocities(&off), "{mml}");
        }
    }
}

#[test]
fn picked_notes_get_picking_noise_just_before_their_attack() {
    for mml in MMLS {
        for rules in rule_sets() {
            let (out, _, articulated, _) = seeded_performance(mml, &rules, 0);
            let picked = articulated
                .iter()
                .filter(|a| matches!(a.articulation, Articulation::SusDown | Articulation::SusUp))
                .count();
            let cc30: Vec<usize> = (0..out.len())
                .filter(|&k| is_cc(&out[k], PICKING_CC))
                .collect();
            assert_eq!(cc30.len(), picked + 1, "{mml} {}", rules.to_json());

            let (reset, sent) = cc30.split_last().unwrap();
            assert_eq!(out[*reset].message[2], PICKING_CC_DEFAULT);
            assert!(out[*reset..].iter().all(|e| !is_played_on(e)));
            for &k in sent {
                let attack = out[k..]
                    .iter()
                    .find(|e| is_played_on(e) && e.seconds == out[k].seconds);
                assert!(
                    attack.is_some(),
                    "CC30 at {} has no attack after it",
                    out[k].seconds
                );
            }
        }
    }
    let (out, _, _, _) = seeded_performance(OCTAVE_RUN, &RuleTable::default(), 0);
    assert!(out.iter().all(|e| !is_cc(e, 31)));
}

#[test]
fn convert_uses_the_fixed_seed_and_moves_notes_off_the_grid() {
    let rules = with_humanize(&RuleTable::default());
    let mml = "o3 l16 e f+ g a b a g f+";
    let converted = convert(&raw(mml), &rules);
    assert_eq!(converted, convert(&raw(mml), &rules));
    let (expected, _, _, _) = seeded_performance(mml, &rules, HUMANIZE_SEED);
    assert_eq!(converted, expected);

    let off_grid = converted
        .iter()
        .filter(|e| is_played_on(e))
        .filter(|e| (e.seconds / 0.125 - (e.seconds / 0.125).round()).abs() > 1e-6)
        .count();
    assert!(off_grid > 4, "off grid {off_grid}");
}

#[test]
fn raw_events_other_than_notes_survive() {
    let volume = TimedMidiEvent {
        seconds: 0.1,
        message: [0xB0, 7, 100],
    };
    let mut events = raw("o3 l8 e f+ g");
    events.push(volume);
    let out = convert(&events, &with_humanize(&RuleTable::default()));
    assert!(out.contains(&volume));
}
