use super::*;

fn hit(note: u8, step: usize, steps: u8) -> DrumHit {
    DrumHit {
        note,
        step,
        steps,
        velocity: DEFAULT_VELOCITY,
    }
}

#[test]
fn hits_drop_out_of_range_cells_clamp_lengths_and_list_in_note_step_order() {
    let pattern = DrumPattern::from_hits([
        hit(42, 3, 2),
        hit(36, 8, 0),
        hit(36, 0, 99),
        hit(200, 0, 1),
        hit(38, DRUM_STEPS, 1),
        hit(42, 3, 5),
    ]);
    assert_eq!(
        pattern.hits().collect::<Vec<_>>(),
        [
            hit(36, 0, DRUM_STEPS as u8),
            hit(36, 8, 1),
            // 同じセルは後勝ち。
            hit(42, 3, 5),
        ]
    );
    assert_eq!(pattern.length(42, 3), Some(5));
    assert_eq!(pattern.length(42, 4), None);
    assert!(!pattern.is_empty());
    assert!(DrumPattern::default().is_empty());
}
