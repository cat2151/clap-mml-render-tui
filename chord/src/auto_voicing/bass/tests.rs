use std::collections::BTreeMap;

use super::*;
use crate::{chord_notes, ChordProgressionCatalog, KEYS};

const CATALOG_JSON: &str = include_str!("../../../testdata/chord-progressions.json");

fn select_for_degrees(degrees: &str, key: &str, key_pitch_class: u8) -> Vec<u8> {
    let chords = chord_notes(degrees, key).unwrap();
    let upper = super::super::upper::select_path(&chords, None).unwrap();
    let roots = chords.iter().map(|notes| notes[0]).collect::<Vec<_>>();
    let lowest_upper = upper
        .iter()
        .map(|notes| *notes.iter().min().unwrap())
        .collect::<Vec<_>>();
    select_path(key_pitch_class, &roots, &lowest_upper, None).unwrap()
}

#[test]
fn c_b_c_uses_the_adjacent_lower_b() {
    assert_eq!(select_for_degrees("I-VII-I", "C", 0), [48, 47, 48]);
}

#[test]
fn one_four_five_one_keeps_the_canonical_roots() {
    assert_eq!(select_for_degrees("I-IV-V-I", "C", 0), [48, 53, 55, 48]);
}

#[test]
fn a_bass_may_double_the_lowest_chord_note_in_unison() {
    // B47 は chord layer の最低音と同音だが候補に残る。B35 へ落とすと続く D まで
    // octave down に引きずられる。
    assert_eq!(
        select_path(0, &[60, 59, 62], &[72, 47, 72], None).unwrap(),
        [48, 47, 50]
    );
}

#[test]
fn an_unavoidable_jump_over_seven_uses_the_lexicographic_optimum() {
    // B47 が chord layer より上なので B35 が必須。続く D は canonical D50
    // より octave-down D38 の方が 7 超過量を減らすため、octave move 数より優先される。
    let selected = select_path(0, &[60, 59, 62], &[72, 46, 72], None).unwrap();
    assert_eq!(selected, [48, 35, 38]);
    assert_eq!(selected[0].abs_diff(selected[1]), 13);
}

#[test]
fn tonic_anchor_and_hard_constraints_can_make_selection_impossible() {
    assert_eq!(select_path(0, &[72], &[80], None), Some(vec![48]));
    assert_eq!(select_path(0, &[60], &[48], None), Some(vec![48]));
    assert_eq!(select_path(0, &[60], &[47], None), None);
    assert_eq!(select_path(0, &[60, 62], &[72], None), None);
}

#[test]
fn seed_only_scores_the_boundary_transition() {
    let roots = [62, 64];
    let lowest_upper = [80, 80];
    let unseeded = select_path(0, &roots, &lowest_upper, None).unwrap();
    let seeded = select_path(0, &roots, &lowest_upper, Some(36)).unwrap();

    assert_eq!(unseeded, [50, 52]);
    assert_eq!(seeded, [38, 40]);
    assert!(seeded.iter().all(|note| (32..=48).contains(note)));
}

#[test]
fn the_full_catalog_obeys_bass_properties_in_all_twelve_keys() {
    let catalog = ChordProgressionCatalog::from_json(CATALOG_JSON).unwrap();

    for (key_pitch_class, key) in KEYS.iter().enumerate() {
        let chords = catalog
            .entries()
            .iter()
            .flat_map(|entry| chord_notes(&entry.degrees, key).unwrap())
            .collect::<Vec<_>>();
        let upper = super::super::upper::select_path(&chords, None).unwrap();
        let roots = chords.iter().map(|notes| notes[0]).collect::<Vec<_>>();
        let lowest_upper = upper
            .iter()
            .map(|notes| *notes.iter().min().unwrap())
            .collect::<Vec<_>>();
        let first = select_path(key_pitch_class as u8, &roots, &lowest_upper, None)
            .unwrap_or_else(|| panic!("Key={key} has no Bass path"));
        let second = select_path(key_pitch_class as u8, &roots, &lowest_upper, None).unwrap();

        assert_eq!(first, second, "Key={key} is not deterministic");
        assert_eq!(first.len(), chords.len(), "Key={key}");
        let min = *first.iter().min().unwrap();
        let max = *first.iter().max().unwrap();
        assert!(max - min <= 16, "Key={key} span={}", max - min);

        let tonic_anchor = 48 + key_pitch_class as u8;
        let mut by_pitch_class = BTreeMap::<u8, Vec<u8>>::new();
        for (index, ((bass, root), lowest_chord)) in
            first.iter().zip(&roots).zip(&lowest_upper).enumerate()
        {
            assert_eq!(bass % 12, root % 12, "Key={key} chord={index}");
            assert!(bass <= lowest_chord, "Key={key} chord={index}");
            if root % 12 == key_pitch_class as u8 {
                assert_eq!(*bass, tonic_anchor, "Key={key} chord={index}");
            }
            by_pitch_class.entry(bass % 12).or_default().push(*bass);
        }
        for (pitch_class, notes) in by_pitch_class {
            let min = *notes.iter().min().unwrap();
            let max = *notes.iter().max().unwrap();
            assert!(
                max - min <= 12,
                "Key={key} pitch_class={pitch_class} spread={}",
                max - min
            );
        }
    }
}
