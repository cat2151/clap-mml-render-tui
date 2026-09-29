use super::*;

fn melody(pitches: &[u8]) -> Vec<Note> {
    pitches
        .iter()
        .enumerate()
        .map(|(column, &pitch)| Note {
            on_seconds: column as f64,
            off_seconds: column as f64 + 1.0,
            channel: 0,
            pitch,
            velocity: 100,
            column,
        })
        .collect()
}

#[test]
fn a_string_holds_notes_within_the_reach() {
    // C4 | F4 G4 | B4 G4 | F4 | C4 | ...（C4→F4 は 5 半音で届かない。隣の弦の同じフレット）
    let n = melody(&[60, 65, 67, 71, 67, 65, 60, 65, 67, 71, 67, 65, 60]);
    assert_eq!(
        strings_by_column(&n),
        [0, 1, 1, 2, 2, 3, 4, 5, 5, 6, 6, 7, 8]
    );
}

#[test]
fn a_major_scale_is_three_notes_per_string() {
    let n = melody(&[60, 62, 64, 65, 67, 69, 71, 72]);
    assert_eq!(strings_by_column(&n), [0, 0, 0, 1, 1, 1, 2, 2]);
}

#[test]
fn a_pentatonic_is_two_notes_per_string_like_the_box() {
    // A マイナーペンタの 5 フレットのボックスを下行: C A | G E | D C | A G
    let n = melody(&[60, 57, 55, 52, 50, 48, 45, 43]);
    assert_eq!(strings_by_column(&n), [0, 0, 1, 1, 2, 2, 3, 3]);
}

#[test]
fn a_chord_is_its_own_string_and_the_next_note_starts_a_new_one() {
    let mut n = melody(&[60, 62]);
    n.push(Note { column: 2, ..n[1] });
    n[2].pitch = 67;
    n.push(Note {
        column: 2,
        pitch: 71,
        ..n[2]
    });
    n.push(Note {
        column: 3,
        pitch: 62,
        ..n[0]
    });
    assert_eq!(strings_by_column(&n), [0, 0, 1, 2]);
    assert!(picks_string(&[0, 0, 1, 2], 3));
    assert!(!picks_string(&[0, 0, 1, 2], 1));
    assert!(picks_string(&[0, 0, 1, 2], 0));
}
