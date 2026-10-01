use super::*;
use crate::{articulate, notes_from_events, RowRule};

use Articulation::{HammerOn, MuteDown, MuteUp, PinchHarmonic, SusDown};

fn notes(mml: &str) -> Vec<Note> {
    notes_from_events(&cmrt_chord::timed_performance(mml).unwrap().events)
}

fn articulations(mml: &str, rules: &RuleTable) -> Vec<Articulation> {
    articulate(&notes(mml), rules)
        .iter()
        .map(|a| a.articulation)
        .collect()
}

fn columns(rule: Rule, columns: &[usize]) -> RuleTable {
    let mut rules = RuleTable::default();
    for &column in columns {
        rules.toggle(column, rule);
    }
    rules
}

#[test]
fn palm_mute_columns_become_mute_down() {
    let rules = columns(Rule::PalmMute, &[1, 2]);
    assert_eq!(
        articulations("o3 l8 e f+ g a", &rules),
        vec![SusDown, MuteDown, MuteDown, SusDown]
    );
}

#[test]
fn palm_mute_keeps_the_economy_stroke() {
    // E2 F#2 G2 は同じ弦で D U D、A2 は高い弦へ移るので D。
    let mut rules = columns(Rule::PalmMute, &[0, 1, 2, 3]);
    rules.toggle_row(RowRule::EconomyPicking);
    assert_eq!(
        articulations("o3 l8 e f+ g a", &rules),
        vec![MuteDown, MuteUp, MuteDown, MuteDown]
    );
}

#[test]
fn pinch_harmonic_columns_become_ph_for_both_strokes() {
    let mut rules = columns(Rule::PinchHarmonic, &[0, 1]);
    rules.toggle_row(RowRule::EconomyPicking);
    assert_eq!(
        articulations("o3 l8 e f+ g a", &rules),
        vec![PinchHarmonic, PinchHarmonic, SusDown, SusDown]
    );
}

#[test]
fn notes_without_a_sample_stay_sus_down() {
    // o6 g = 79 は 30〜76 の外。
    assert_eq!(notes("o6 g")[0].pitch, 79);
    for rule in [Rule::PalmMute, Rule::PinchHarmonic] {
        assert_eq!(articulations("o6 g", &columns(rule, &[0])), vec![SusDown]);
    }
    assert_eq!(notes("o2 f+")[0].pitch, 30);
    assert_eq!(
        articulations("o2 f+", &columns(Rule::PalmMute, &[0])),
        vec![MuteDown]
    );
}

#[test]
fn hammer_pull_notes_stay_as_they_are() {
    // 自動ハンマリングで 2 音目が H/P になる列に PalmMute を ON にしても H/P のまま。
    let mut rules = columns(Rule::PalmMute, &[1]);
    rules.toggle_row(RowRule::AutoHammerPull);
    assert_eq!(articulations("o3 l8 e f+", &rules), vec![SusDown, HammerOn]);
}

#[test]
fn chord_columns_are_voiced_note_by_note() {
    assert_eq!(
        articulations("o3 l8 'egb'", &columns(Rule::PalmMute, &[0])),
        vec![MuteDown, MuteDown, MuteDown]
    );
}

/// 1 列 1 音の、間隔 0.25 秒の音。
fn single_notes(pitches: &[u8]) -> Vec<Note> {
    pitches
        .iter()
        .enumerate()
        .map(|(column, &pitch)| Note {
            on_seconds: column as f64 * 0.25,
            off_seconds: column as f64 * 0.25 + 0.25,
            channel: 0,
            pitch,
            velocity: 100,
            column,
        })
        .collect()
}

/// (ルール, ダウンの写し先, KS, 音域の下端, 音域の上端)。
const LIST_RULES: [(Rule, Articulation, u8, u8, u8); 11] = [
    (
        Rule::NaturalHarmonics,
        Articulation::NaturalHarmonics,
        9,
        35,
        88,
    ),
    (Rule::Brushing, Articulation::BrushDown, 11, 31, 51),
    (Rule::FretMute, Articulation::MuteFretDown, 14, 31, 88),
    (Rule::SlideOut, Articulation::SlideOut, 28, 32, 88),
    (Rule::PseudoLegato, Articulation::PseudoLegato, 29, 30, 88),
    (Rule::Portamento, Articulation::Portamento, 96, 30, 88),
    (Rule::TrillHalf, Articulation::TrillHalf, 99, 30, 88),
    (Rule::TrillWhole, Articulation::TrillWhole, 100, 30, 88),
    (
        Rule::TrillMinorThird,
        Articulation::TrillMinorThird,
        101,
        30,
        88,
    ),
    (
        Rule::TrillMajorThird,
        Articulation::TrillMajorThird,
        102,
        30,
        88,
    ),
    (
        Rule::UnisonBendAuto,
        Articulation::UnisonBendAuto,
        94,
        60,
        84,
    ),
];

#[test]
fn list_rules_voice_only_the_pitches_that_have_a_sample() {
    for (rule, voiced, keyswitch, low, high) in LIST_RULES {
        assert_eq!(voiced.keyswitch(), keyswitch, "{rule:?}");
        let notes = single_notes(&[low - 1, low, high, high + 1]);
        let mut articulations = vec![SusDown; notes.len()];
        apply_voicing_rules(&notes, &columns(rule, &[0, 1, 2, 3]), &mut articulations);
        assert_eq!(
            articulations,
            vec![SusDown, voiced, voiced, SusDown],
            "{rule:?}: 音域の外は元のまま"
        );
    }
}

#[test]
fn brushing_and_fret_mute_keep_the_stroke() {
    for (rule, down, up) in [
        (
            Rule::Brushing,
            Articulation::BrushDown,
            Articulation::BrushUp,
        ),
        (
            Rule::FretMute,
            Articulation::MuteFretDown,
            Articulation::MuteFretUp,
        ),
    ] {
        let mut rules = columns(rule, &[0, 1, 2, 3]);
        rules.toggle_row(RowRule::EconomyPicking);
        // E2 F#2 G2 は同じ弦で D U D、A2 は高い弦へ移るので D。
        assert_eq!(
            articulations("o3 l8 e f+ g a", &rules),
            vec![down, up, down, down],
            "{rule:?}"
        );
    }
    let mut rules = columns(Rule::NaturalHarmonics, &[0, 1]);
    rules.toggle_row(RowRule::EconomyPicking);
    assert_eq!(
        articulations("o3 l8 e f+ g a", &rules)[..2],
        [
            Articulation::NaturalHarmonics,
            Articulation::NaturalHarmonics
        ]
    );
}

#[test]
fn only_pseudo_legato_and_portamento_voice_hammer_pull_notes() {
    for (rule, voiced, _, _, _) in LIST_RULES {
        let mut rules = columns(rule, &[1]);
        rules.toggle_row(RowRule::AutoHammerPull);
        let expected = if matches!(rule, Rule::PseudoLegato | Rule::Portamento) {
            voiced
        } else {
            HammerOn
        };
        assert_eq!(
            articulations("o3 l8 e f+", &rules),
            vec![SusDown, expected],
            "{rule:?}"
        );
    }
}

#[test]
fn slide_in_needs_the_lowest_pitch_of_its_width() {
    assert_eq!(Articulation::SlideIn.keyswitch(), 27);
    // 幅 w の下端: 1〜6 は 31 + w、7 は sfz の区切りどおり 32。
    for (width, lowest) in [
        (1, 32),
        (2, 33),
        (3, 34),
        (4, 35),
        (5, 36),
        (6, 37),
        (7, 32),
    ] {
        for (pitch, expected) in [
            (lowest - 1, SusDown),
            (lowest, Articulation::SlideIn),
            (88, Articulation::SlideIn),
            (89, SusDown),
        ] {
            let notes = single_notes(&[pitch - width, pitch]);
            let mut articulations = vec![SusDown; 2];
            apply_voicing_rules(&notes, &columns(Rule::SlideIn, &[1]), &mut articulations);
            assert_eq!(articulations[1], expected, "幅 {width} の {pitch}");
        }
    }
}

#[test]
fn slide_in_width_is_the_interval_clamped_to_one_through_seven() {
    assert_eq!(slide_in_width(None), 1);
    assert_eq!(slide_in_width(Some(0)), 1);
    assert_eq!(slide_in_width(Some(3)), 3);
    assert_eq!(slide_in_width(Some(11)), SLIDE_MAX_SEMITONES);
    // 先頭の列は幅 1 なので、下端は 32。
    let mut articulations = vec![SusDown; 2];
    let notes = single_notes(&[31, 32]);
    apply_voicing_rules(&notes, &columns(Rule::SlideIn, &[0]), &mut articulations);
    assert_eq!(articulations[0], SusDown);
    let notes = single_notes(&[32, 40]);
    apply_voicing_rules(&notes, &columns(Rule::SlideIn, &[0]), &mut articulations);
    assert_eq!(articulations[0], Articulation::SlideIn);
}
