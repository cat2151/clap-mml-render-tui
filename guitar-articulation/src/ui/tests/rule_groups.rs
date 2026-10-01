use std::collections::BTreeSet;

use crossterm::event::KeyCode;

use cmrt_tui_core::theme::{
    MONOKAI_CYAN, MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_PURPLE, MONOKAI_YELLOW,
};

use super::{key, label_fg, render, screen_with_mml};
use crate::ui::{layout_for, RuleGroup, RULE_LIST_KEY, RULE_ROWS};
use crate::Rule;

/// 全 `Rule`。`variant_index` の match が網羅なので、variant を足すとここも直すまでコンパイルが通らない。
const ALL_RULES: [Rule; 32] = [
    Rule::HammerPull,
    Rule::PalmMute,
    Rule::PinchHarmonic,
    Rule::Slide,
    Rule::Choke,
    Rule::Vibrato,
    Rule::PickScratch,
    Rule::NaturalHarmonics,
    Rule::Brushing,
    Rule::FretMute,
    Rule::SlideOut,
    Rule::PseudoLegato,
    Rule::Portamento,
    Rule::SlideIn,
    Rule::TrillHalf,
    Rule::TrillWhole,
    Rule::TrillMinorThird,
    Rule::TrillMajorThird,
    Rule::UnisonBendAuto,
    Rule::UnisonBendManual,
    Rule::ChromaticRun,
    Rule::SlideFxDown,
    Rule::SlideFxUp,
    Rule::SlideFxWow,
    Rule::EffectHello,
    Rule::EffectResonance,
    Rule::EffectSlideNoise,
    Rule::EffectHardStop,
    Rule::LongExtra,
    Rule::PowerChord,
    Rule::PositionRelease,
    Rule::AutoSlideOut,
];

fn variant_index(rule: Rule) -> usize {
    match rule {
        Rule::HammerPull => 0,
        Rule::PalmMute => 1,
        Rule::PinchHarmonic => 2,
        Rule::Slide => 3,
        Rule::Choke => 4,
        Rule::Vibrato => 5,
        Rule::PickScratch => 6,
        Rule::NaturalHarmonics => 7,
        Rule::Brushing => 8,
        Rule::FretMute => 9,
        Rule::SlideOut => 10,
        Rule::PseudoLegato => 11,
        Rule::Portamento => 12,
        Rule::SlideIn => 13,
        Rule::TrillHalf => 14,
        Rule::TrillWhole => 15,
        Rule::TrillMinorThird => 16,
        Rule::TrillMajorThird => 17,
        Rule::UnisonBendAuto => 18,
        Rule::UnisonBendManual => 19,
        Rule::ChromaticRun => 20,
        Rule::SlideFxDown => 21,
        Rule::SlideFxUp => 22,
        Rule::SlideFxWow => 23,
        Rule::EffectHello => 24,
        Rule::EffectResonance => 25,
        Rule::EffectSlideNoise => 26,
        Rule::EffectHardStop => 27,
        Rule::LongExtra => 28,
        Rule::PowerChord => 29,
        Rule::PositionRelease => 30,
        Rule::AutoSlideOut => 31,
    }
}

#[test]
fn every_rule_has_exactly_one_row() {
    for (index, rule) in ALL_RULES.into_iter().enumerate() {
        assert_eq!(variant_index(rule), index, "{rule:?}");
        let rows = RULE_ROWS.iter().filter(|row| row.rule == rule).count();
        assert_eq!(rows, 1, "{rule:?}");
    }
    assert_eq!(RULE_ROWS.len(), ALL_RULES.len());
}

#[test]
fn groups_run_in_order_attack_release_then_pitch_then_trick() {
    let mut groups: Vec<RuleGroup> = RULE_ROWS.iter().map(|row| row.group).collect();
    groups.dedup();
    assert_eq!(
        groups,
        [RuleGroup::AttackRelease, RuleGroup::Pitch, RuleGroup::Trick]
    );
    let names: Vec<&str> = RULE_ROWS.iter().map(|row| row.name).collect();
    assert_eq!(
        names,
        [
            "hammer/pull",
            "palm mute",
            "pinch harmonic",
            "harmonics",
            "brush",
            "fret mute",
            "long/extra",
            "power chord",
            "position rel",
            "slide up/down",
            "bend",
            "vibrato",
            "slide in",
            "slide out",
            "auto slide out",
            "pseudo legato",
            "portamento",
            "trill half",
            "trill whole",
            "trill min3",
            "trill maj3",
            "unison bend",
            "unison manual",
            "pick scratch",
            "chromatic run",
            "slide fx down",
            "slide fx up",
            "slide fx wow",
            "fx hello",
            "fx resonance",
            "fx slide noise",
            "fx hard stop",
        ]
    );
}

#[test]
fn overlay_keys_are_unique_letters_that_reuse_the_main_screen_keys() {
    let keys: String = RULE_ROWS.iter().map(|row| row.overlay_key).collect();
    assert_eq!(keys, "ampbdefinocvqrstuwxyzABgCDEFGHIJ");
    let unique: BTreeSet<char> = keys.chars().collect();
    assert_eq!(unique.len(), keys.chars().count(), "{keys}");
    for row in RULE_ROWS {
        assert!(row.overlay_key.is_ascii_alphabetic(), "{row:?}");
        assert!(!"hjkl".contains(row.overlay_key), "{row:?}");
        if row.key != RULE_LIST_KEY && row.key.is_ascii_alphabetic() {
            assert_eq!(row.overlay_key, row.key, "{row:?}");
        }
    }
    let main_keys: String = RULE_ROWS
        .iter()
        .filter(|row| row.key != RULE_LIST_KEY && row.key.is_ascii_alphabetic())
        .map(|row| row.key)
        .collect();
    assert_eq!(main_keys, "ampcvg");
}

#[test]
fn group_colors_follow_the_spec() {
    assert_eq!(RuleGroup::Row.color(), MONOKAI_YELLOW);
    assert_eq!(RuleGroup::AttackRelease.color(), MONOKAI_GREEN);
    assert_eq!(RuleGroup::Pitch.color(), MONOKAI_CYAN);
    assert_eq!(RuleGroup::Trick.color(), MONOKAI_PURPLE);
}

#[test]
fn matrix_labels_are_painted_by_group() {
    let screen = screen_with_mml("o3 l8 e f+ g");
    let buffer = render(&screen);
    let matrix = layout_for(buffer.area, &screen).matrix;

    assert_eq!(label_fg(&buffer, matrix, "E2"), MONOKAI_GRAY);
    assert_eq!(label_fg(&buffer, matrix, "d:humanize"), MONOKAI_YELLOW);
    assert_eq!(
        label_fg(&buffer, matrix, "s:auto hammer/pull"),
        MONOKAI_YELLOW
    );
    assert_eq!(label_fg(&buffer, matrix, "t:KS"), MONOKAI_GRAY);
    assert_eq!(label_fg(&buffer, matrix, "v:vibrato"), MONOKAI_CYAN);
    assert_eq!(label_fg(&buffer, matrix, "t:long/extra"), MONOKAI_GREEN);
    assert_eq!(label_fg(&buffer, matrix, "t:power chord"), MONOKAI_GREEN);
    assert_eq!(label_fg(&buffer, matrix, "t:release"), MONOKAI_GRAY);
}

#[test]
fn overlay_labels_use_the_overlay_keys_and_are_painted_by_group() {
    let mut screen = screen_with_mml("o3 l8 e f+ g");
    screen.handle_key_event(key(KeyCode::Char('t')));
    let buffer = render(&screen);
    let all = buffer.area;

    // KS のピッチと飛び道具の行は幅に収まらないので、左寄りの項目で見る。
    assert_eq!(label_fg(&buffer, all, "b:harmonics"), MONOKAI_GREEN);
    assert_eq!(label_fg(&buffer, all, "o:slide up/down"), MONOKAI_CYAN);
    assert_eq!(label_fg(&buffer, all, "q:slide in"), MONOKAI_CYAN);
    assert_eq!(label_fg(&buffer, all, "v:vibrato"), MONOKAI_CYAN);
    assert_eq!(label_fg(&buffer, all, "f:long/extra"), MONOKAI_GREEN);
    assert_eq!(label_fg(&buffer, all, "s:auto slide out"), MONOKAI_CYAN);
    assert_eq!(label_fg(&buffer, all, "C:chromatic run"), MONOKAI_PURPLE);
    assert_eq!(label_fg(&buffer, all, "D:slide fx down"), MONOKAI_PURPLE);
}
