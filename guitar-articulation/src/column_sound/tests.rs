use super::*;
use crate::{articulate, convert, notes_from_events, RowRule};

fn raw(mml: &str) -> Vec<TimedMidiEvent> {
    cmrt_chord::timed_performance(mml).unwrap().events
}

fn rule_on(column: usize, rule: Rule) -> RuleTable {
    let mut rules = RuleTable::default();
    rules.toggle(column, rule);
    rules
}

/// 列ルールを掛けた (奏法, 鳴らす音高)。
fn sounded(mml: &str, rules: &RuleTable) -> Vec<(Articulation, Option<u8>)> {
    articulate(&notes_from_events(&raw(mml)), rules)
        .iter()
        .map(|a| (a.articulation, a.pitch))
        .collect()
}

/// 演奏音の note on の (音高, velocity)。KS を除く。
fn played(events: &[TimedMidiEvent]) -> Vec<(u8, u8)> {
    events
        .iter()
        .filter(|e| is_note_on(&e.message) && Articulation::from_keyswitch(e.message[1]).is_none())
        .map(|e| (e.message[1], e.message[2]))
        .collect()
}

#[test]
fn fold_pitch_moves_by_octaves_into_the_range() {
    for (pitch, range, folded) in [
        (30, 30..=41, 30),
        (41, 30..=41, 41),
        (42, 30..=41, 30),
        (29, 30..=41, 41),
        (0, 42..=66, 48),
        (67, 42..=66, 55),
        (127, 30..=54, 43),
    ] {
        assert_eq!(
            fold_pitch(pitch, range.clone()),
            folded,
            "{pitch} {range:?}"
        );
    }
}

#[test]
fn keyswitches_of_the_effect_articulations() {
    for (articulation, key) in [
        (Articulation::ChromaticRun, 4),
        (Articulation::SlideFxDown, 5),
        (Articulation::SlideFxUp, 6),
        (Articulation::SlideFxWow, 7),
    ] {
        assert_eq!(articulation.keyswitch(), key);
        assert_eq!(Articulation::from_keyswitch(key), Some(articulation));
    }
}

#[test]
fn chromatic_run_folds_to_a_phrase_key_and_skips_the_keys_without_a_phrase() {
    // C4 = 60 → 36、A1 = 33 → 33、F#1 = 30 → 30、D2 = 38 → 38。
    for (mml, key) in [("o5 c", 36), ("o2 a", 33), ("o2 f+", 30), ("o3 d", 38)] {
        assert_eq!(
            sounded(mml, &rule_on(0, Rule::ChromaticRun)),
            vec![(Articulation::ChromaticRun, Some(key))],
            "{mml}"
        );
    }
    // D#2〜F2（39〜41）と、畳んでそこへ来る音（D#4 = 63 → 39）は効かず、元のまま。
    for (mml, pitch) in [("o3 d+", 39), ("o3 f", 41), ("o5 d+", 63)] {
        assert_eq!(
            sounded(mml, &rule_on(0, Rule::ChromaticRun)),
            vec![(Articulation::SusDown, Some(pitch))],
            "{mml}"
        );
    }
}

#[test]
fn slide_fx_rules_fold_into_their_ranges() {
    // C5 = 72 は Down（42〜66）へ 60、Up / Wow（30〜54）へ 48。C2 = 36 は Down へ 48、Up へ 36。
    for (rule, articulation, high, low) in [
        (Rule::SlideFxDown, Articulation::SlideFxDown, 60, 48),
        (Rule::SlideFxUp, Articulation::SlideFxUp, 48, 36),
        (Rule::SlideFxWow, Articulation::SlideFxWow, 48, 36),
    ] {
        assert_eq!(
            sounded("o6 c o3 c", &{
                let mut rules = rule_on(0, rule);
                rules.toggle(1, rule);
                rules
            }),
            vec![(articulation, Some(high)), (articulation, Some(low))],
            "{rule:?}"
        );
    }
}

#[test]
fn effect_rules_sound_notes_zero_to_three_without_a_keyswitch() {
    for (rule, note) in [
        (Rule::EffectHello, EFFECT_HELLO_PITCH),
        (Rule::EffectResonance, EFFECT_RESONANCE_PITCH),
        (Rule::EffectSlideNoise, EFFECT_SLIDE_NOISE_PITCH),
        (Rule::EffectHardStop, EFFECT_HARD_STOP_PITCH),
    ] {
        let out = convert(&raw("o5 l4 c 'egb' c"), &rule_on(1, rule));
        let pitches: Vec<u8> = played(&out).iter().map(|(p, _)| *p).collect();
        assert_eq!(pitches, vec![60, note, 60], "{rule:?}");
        // 効果音の列は奏法を変えないので、KS は先頭の既定 1 つだけ。
        let keyswitches: Vec<u8> = out
            .iter()
            .filter(|e| {
                is_note_on(&e.message) && Articulation::from_keyswitch(e.message[1]).is_some()
            })
            .map(|e| e.message[1])
            .collect();
        assert_eq!(
            keyswitches,
            vec![Articulation::SusDown.keyswitch()],
            "{rule:?}"
        );
        let offs = out
            .iter()
            .filter(|e| is_note_off(&e.message) && [note, 64, 67, 71].contains(&e.message[1]))
            .count();
        assert_eq!(offs, 1, "{rule:?} {out:?}");
    }
}

#[test]
fn a_chord_column_sounds_only_its_lowest_note() {
    for rule in [
        Rule::ChromaticRun,
        Rule::SlideFxDown,
        Rule::SlideFxUp,
        Rule::SlideFxWow,
    ] {
        let pitches: Vec<Option<u8>> = sounded("o3 'dfa'", &rule_on(0, rule))
            .iter()
            .map(|(_, pitch)| *pitch)
            .collect();
        assert_eq!(pitches.iter().flatten().count(), 1, "{rule:?} {pitches:?}");
        assert!(pitches[0].is_some(), "{rule:?} {pitches:?}");
    }
}

#[test]
fn effect_columns_keep_the_mml_velocity_under_dynamics() {
    // エコノミーピッキングで強弱が付いても、2 音目（アクセントでない）の効果音とスライドエフェクトは弱めない。
    for (rule, sounding) in [
        (Rule::SlideFxDown, 48),
        (Rule::EffectHello, EFFECT_HELLO_PITCH),
    ] {
        let mut rules = rule_on(1, rule);
        rules.toggle_row(RowRule::EconomyPicking);
        let out = convert(&raw("o4 l8 v10 e c d"), &rules);
        let velocity = played(&out)
            .into_iter()
            .find(|(pitch, _)| *pitch == sounding)
            .map(|(_, velocity)| velocity);
        let plain = played(&raw("o4 l8 v10 e c d"))[1].1;
        assert_eq!(velocity, Some(plain), "{rule:?} {out:?}");
    }
}

#[test]
fn the_new_rules_are_exclusive_and_round_trip_through_json() {
    let mut rules = rule_on(0, Rule::PalmMute);
    rules.toggle(0, Rule::EffectHardStop);
    rules.toggle(0, Rule::Vibrato);
    assert!(!rules.is_on(0, Rule::PalmMute));
    assert_eq!(
        rules.to_json(),
        r#"{"columns":{"0":["vibrato","effect_hard_stop"]},"rows":[]}"#
    );
    assert_eq!(RuleTable::from_json(&rules.to_json()).unwrap(), rules);
}
