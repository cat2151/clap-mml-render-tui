use super::*;

#[test]
fn note_name_uses_the_mml_octave_numbering() {
    assert_eq!(note_name(60), "c5");
    assert_eq!(note_name(61), "c+5");
    assert_eq!(note_name(72), "c6");
}
