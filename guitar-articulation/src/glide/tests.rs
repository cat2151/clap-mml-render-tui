use super::*;
use crate::{articulate, notes_from_events, RowRule};

use Articulation::{
    BendHalf, BendWhole, BendWholeHalf, HammerOn, SlideDown, SlideUp, SusDown, SusUp,
};

fn notes(mml: &str) -> Vec<Note> {
    notes_from_events(&cmrt_chord::timed_performance(mml).unwrap().events)
}

fn articulations(mml: &str, rules: &RuleTable) -> Vec<Articulation> {
    articulate(&notes(mml), rules)
        .iter()
        .map(|a| a.articulation)
        .collect()
}

fn on(rule: Rule, columns: &[usize]) -> RuleTable {
    let mut rules = RuleTable::default();
    for &column in columns {
        rules.toggle(column, rule);
    }
    rules
}

#[test]
fn rising_minor_third_slides_up_or_bends_a_whole_and_half() {
    assert_eq!(
        articulations("o3 l8 e g", &on(Rule::Slide, &[1])),
        vec![SusDown, SlideUp]
    );
    assert_eq!(
        articulations("o3 l8 e g", &on(Rule::Choke, &[1])),
        vec![SusDown, BendWholeHalf]
    );
}

#[test]
fn falling_minor_third_slides_down_and_does_not_bend() {
    assert_eq!(
        articulations("o3 l8 g e", &on(Rule::Slide, &[1])),
        vec![SusDown, SlideDown]
    );
    assert_eq!(
        articulations("o3 l8 g e", &on(Rule::Choke, &[1])),
        vec![SusDown, SusDown]
    );
}

#[test]
fn bend_width_follows_the_interval() {
    assert_eq!(
        articulations("o3 l8 e f", &on(Rule::Choke, &[1])),
        vec![SusDown, BendHalf]
    );
    assert_eq!(
        articulations("o3 l8 e f+", &on(Rule::Choke, &[1])),
        vec![SusDown, BendWhole]
    );
    assert_eq!(
        articulations("o3 l8 e g+", &on(Rule::Choke, &[1])),
        vec![SusDown, SusDown],
        "4 半音はチョーキングにしない"
    );
}

#[test]
fn intervals_wider_than_seven_semitones_slide_but_unisons_do_not() {
    // C3 → B3 は +11。7 半音の幅で滑る。
    assert_eq!(
        articulations("o3 l8 c b", &on(Rule::Slide, &[1])),
        vec![SusDown, SlideUp]
    );
    assert_eq!(
        articulations("o3 l8 e e", &on(Rule::Slide, &[1])),
        vec![SusDown, SusDown]
    );
    // C3 → G3 は +7（3 音半）で滑る。
    assert_eq!(
        articulations("o3 l8 c g", &on(Rule::Slide, &[1])),
        vec![SusDown, SlideUp]
    );
}

#[test]
fn an_octave_slides_up_and_down() {
    assert_eq!(
        articulations("o3 l8 c < c", &on(Rule::Slide, &[1])),
        vec![SusDown, SlideUp]
    );
    assert_eq!(
        articulations("o4 l8 c > c", &on(Rule::Slide, &[1])),
        vec![SusDown, SlideDown]
    );
}

#[test]
fn wide_slides_use_the_range_of_the_seven_semitone_sample() {
    // Slide_Down の 7 半音は 81 まで。
    assert_eq!(notes("o6 a")[0].pitch, 81);
    assert_eq!(
        articulations("o7 l8 a > a", &on(Rule::Slide, &[1])),
        vec![SusDown, SlideDown]
    );
    assert_eq!(
        articulations("o7 l8 a+ > a+", &on(Rule::Slide, &[1])),
        vec![SusDown, SusDown]
    );
    // Slide_Up の 7 半音は 38 から。
    assert_eq!(notes("o3 d")[0].pitch, 38);
    assert_eq!(
        articulations("o2 l8 d < d", &on(Rule::Slide, &[1])),
        vec![SusDown, SlideUp]
    );
    assert_eq!(
        articulations("o2 l8 c+ < c+", &on(Rule::Slide, &[1])),
        vec![SusDown, SusDown]
    );
}

#[test]
fn bend_ignores_wide_intervals() {
    // C3 → G3 は +7。bend は 1〜3 半音だけ。
    assert_eq!(
        articulations("o3 l8 c g", &on(Rule::Choke, &[1])),
        vec![SusDown, SusDown]
    );
}

#[test]
fn slides_need_a_sample_for_their_width() {
    // 30 → 31 は無い（半音の Slide_Up は 32 から）。31 → 32 は在る。
    assert_eq!(notes("o2 f+")[0].pitch, 30);
    assert_eq!(
        articulations("o2 l8 f+ g", &on(Rule::Slide, &[1])),
        vec![SusDown, SusDown]
    );
    assert_eq!(
        articulations("o2 l8 g g+", &on(Rule::Slide, &[1])),
        vec![SusDown, SlideUp]
    );
    // 88 → 87: 半音の Slide_Down は 87 まで。89 → 88 は無い。
    assert_eq!(notes("o7 e")[0].pitch, 88);
    assert_eq!(
        articulations("o7 l8 e d+", &on(Rule::Slide, &[1])),
        vec![SusDown, SlideDown]
    );
    assert_eq!(
        articulations("o7 l8 f e", &on(Rule::Slide, &[1])),
        vec![SusDown, SusDown]
    );
}

#[test]
fn bends_need_a_sample_at_the_destination() {
    // 90 まで。
    assert_eq!(
        articulations("o7 l8 f f+", &on(Rule::Choke, &[1])),
        vec![SusDown, BendHalf]
    );
    assert_eq!(
        articulations("o7 l8 f+ g", &on(Rule::Choke, &[1])),
        vec![SusDown, SusDown]
    );
}

#[test]
fn chords_and_the_first_column_stay() {
    assert_eq!(
        articulations("o3 l8 'egb' <c", &on(Rule::Slide, &[0, 1])),
        vec![SusDown; 4]
    );
    assert_eq!(
        articulations("o3 l8 e 'gb'", &on(Rule::Choke, &[1])),
        vec![SusDown; 3]
    );
}

#[test]
fn a_column_rule_wins_over_auto_hammer_pull() {
    let mut rules = RuleTable::default();
    rules.toggle_row(RowRule::AutoHammerPull);
    assert_eq!(
        articulations("o3 l8 e g", &rules),
        vec![SusDown, HammerOn],
        "前提: 自動ハンマリングが 2 音目を H/P にする"
    );
    rules.toggle(1, Rule::Slide);
    assert_eq!(articulations("o3 l8 e g", &rules), vec![SusDown, SlideUp]);
}

#[test]
fn economy_picking_does_not_count_a_slide_as_a_pick() {
    let mut rules = on(Rule::Slide, &[1]);
    rules.toggle_row(RowRule::EconomyPicking);
    // 同じ弦の E2 (D) → F#2 (slide) → G2 は、前にピッキングした E2 の逆で U。
    assert_eq!(
        articulations("o3 l8 e f+ g a", &rules),
        vec![SusDown, SlideUp, SusUp, SusDown]
    );
}

#[test]
fn slide_semitones_is_the_interval_size_from_a_single_previous_note() {
    let notes = notes("o3 l8 g e 'gb'");
    assert_eq!(slide_semitones(&notes, 0), None);
    assert_eq!(slide_semitones(&notes, 1), Some(3));
    assert_eq!(slide_semitones(&notes, 2), None);
}
