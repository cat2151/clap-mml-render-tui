use super::*;

use crate::chord_notes;

#[test]
fn selection_is_deterministic_and_keeps_every_pitch_class() {
    let chords = chord_notes("IM7-IVM7-IIm7-V7", "C").unwrap();
    let first = select_path(&chords, None).unwrap();
    let second = select_path(&chords, None).unwrap();

    assert_eq!(first, second);
    for (source, voiced) in chords.iter().zip(first) {
        let mut source_classes = source.iter().map(|note| note % 12).collect::<Vec<_>>();
        let mut voiced_classes = voiced.iter().map(|note| note % 12).collect::<Vec<_>>();
        source_classes.sort_unstable();
        source_classes.dedup();
        voiced_classes.sort_unstable();
        voiced_classes.dedup();
        assert_eq!(voiced_classes, source_classes);
    }
}

#[test]
fn seed_changes_the_boundary_transition() {
    let chords = chord_notes("IV-V", "C").unwrap();
    let low = select_path(&chords, Some(&[60, 64, 67])).unwrap();
    let high = select_path(&chords, Some(&[72, 76, 79])).unwrap();

    assert!(low[0].iter().max() < high[0].iter().max());
}
