use super::*;
use crate::notes_from_events;

fn notes(mml: &str) -> Vec<Note> {
    notes_from_events(&cmrt_chord::timed_performance(mml).unwrap().events)
}

#[test]
fn top_marks_head_and_peaks() {
    // c e d f c: 頂点は e と f、谷は d。
    let notes = notes("l16cedfc");
    assert_eq!(
        accents(&notes, AccentPattern::Top),
        vec![true, true, false, true, false]
    );
}

#[test]
fn bottom_marks_head_and_valleys() {
    let notes = notes("l16cedfc");
    assert_eq!(
        accents(&notes, AccentPattern::Bottom),
        vec![true, false, true, false, false]
    );
}

#[test]
fn both_marks_head_peaks_and_valleys() {
    let notes = notes("l16cedfc");
    assert_eq!(
        accents(&notes, AccentPattern::Both),
        vec![true, true, true, true, false]
    );
}

#[test]
fn head_is_accented_even_when_it_is_no_turn() {
    let notes = notes("l16edc");
    assert_eq!(
        accents(&notes, AccentPattern::Bottom),
        vec![true, false, false]
    );
}

#[test]
fn next_cycles_through_three_patterns() {
    let start = AccentPattern::default();
    assert_eq!(start, AccentPattern::Top);
    assert_eq!(start.next(), AccentPattern::Bottom);
    assert_eq!(start.next().next(), AccentPattern::Both);
    assert_eq!(start.next().next().next(), AccentPattern::Top);
}
