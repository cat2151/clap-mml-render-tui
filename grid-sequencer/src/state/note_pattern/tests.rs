use super::*;

fn steps(pattern: &NotePattern) -> Vec<NoteStep> {
    pattern.steps().to_vec()
}

#[test]
fn default_is_sixteen_rests() {
    assert_eq!(
        steps(&NotePattern::default()),
        vec![NoteStep::Rest; GRID_STEPS]
    );
}

#[test]
fn draw_span_builds_attack_and_ties_and_reports_real_changes_only() {
    let mut pattern = NotePattern::default();
    assert!(pattern.draw_span(3, 6));
    assert_eq!(pattern.step(3), Some(NoteStep::Attack));
    assert_eq!(pattern.step(4), Some(NoteStep::Tie));
    assert_eq!(pattern.step(6), Some(NoteStep::Tie));
    assert_eq!(pattern.attack_len(3), Some(4));
    assert!(!pattern.draw_span(3, 6));
}

#[test]
fn draw_span_supports_one_step_clamps_end_and_replaces_crossed_events() {
    let mut pattern = NotePattern::default();
    assert!(pattern.draw_span(5, 5));
    assert_eq!(pattern.attack_len(5), Some(1));
    assert!(pattern.draw_span(14, usize::MAX));
    assert_eq!(pattern.attack_len(14), Some(2));

    let mut pattern = NotePattern::from_steps([
        NoteStep::Rest,
        NoteStep::Attack,
        NoteStep::Tie,
        NoteStep::Attack,
        NoteStep::Tie,
        NoteStep::Tie,
    ]);
    assert!(pattern.draw_span(1, 3));
    assert_eq!(pattern.attack_len(1), Some(3));
    assert_eq!(pattern.step(4), Some(NoteStep::Rest));
    assert_eq!(pattern.step(5), Some(NoteStep::Rest));
    assert!(!pattern.draw_span(GRID_STEPS, GRID_STEPS));
}

#[test]
fn erase_from_attack_or_tie_removes_only_that_event() {
    let original = [
        NoteStep::Attack,
        NoteStep::Tie,
        NoteStep::Tie,
        NoteStep::Attack,
        NoteStep::Attack,
        NoteStep::Tie,
    ];
    let mut from_tie = NotePattern::from_steps(original);
    assert!(from_tie.erase_note_at(1));
    assert_eq!(from_tie.step(0), Some(NoteStep::Rest));
    assert_eq!(from_tie.step(3), Some(NoteStep::Attack));

    let mut from_attack = NotePattern::from_steps(original);
    assert!(from_attack.erase_note_at(3));
    assert_eq!(from_attack.step(3), Some(NoteStep::Rest));
    assert_eq!(from_attack.step(4), Some(NoteStep::Attack));
    assert_eq!(from_attack.attack_len(4), Some(2));
    assert!(!from_attack.erase_note_at(GRID_STEPS));
}

#[test]
fn orphan_ties_are_normalized_after_start_and_rest() {
    let pattern = NotePattern::from_steps([
        NoteStep::Tie,
        NoteStep::Tie,
        NoteStep::Attack,
        NoteStep::Tie,
        NoteStep::Rest,
        NoteStep::Tie,
    ]);
    assert_eq!(
        &pattern.steps()[..6],
        &[
            NoteStep::Rest,
            NoteStep::Rest,
            NoteStep::Attack,
            NoteStep::Tie,
            NoteStep::Rest,
            NoteStep::Rest,
        ]
    );
    assert_eq!(pattern.sounding_end(), Some(3));
}
