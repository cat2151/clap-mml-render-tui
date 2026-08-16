use super::*;

fn pitches(prefix: &str) -> Option<Vec<u8>> {
    notes_at_prefix(prefix).map(|notes| notes.pitches)
}

#[test]
fn empty_prefix_has_no_note() {
    assert_eq!(notes_at_prefix(""), None);
}

#[test]
fn bare_note_sounds_middle_c() {
    assert_eq!(pitches("c"), Some(vec![60]));
}

#[test]
fn typing_each_letter_advances_the_note() {
    assert_eq!(pitches("c"), Some(vec![60]));
    assert_eq!(pitches("cd"), Some(vec![62]));
    assert_eq!(pitches("cde"), Some(vec![64]));
}

#[test]
fn moving_the_cursor_left_returns_to_the_earlier_note() {
    assert_eq!(notes_at_cursor("cde", 3).unwrap().pitches, vec![64]);
    assert_eq!(notes_at_cursor("cde", 2).unwrap().pitches, vec![62]);
    assert_eq!(notes_at_cursor("cde", 1).unwrap().pitches, vec![60]);
    assert_eq!(notes_at_cursor("cde", 0), None);
}

#[test]
fn modifier_shifts_the_pitch() {
    assert_eq!(pitches("c+"), Some(vec![61]));
    assert_eq!(pitches("c-"), Some(vec![59]));
}

#[test]
fn octave_and_modifier_accumulate_across_the_prefix() {
    // `<` はオクターブ上げ。カーソルより前の状態がそのまま効く。
    assert_eq!(pitches("c+1<c+"), Some(vec![73]));
}

#[test]
fn note_length_does_not_add_a_note() {
    let plain = notes_at_prefix("cde").unwrap();
    let with_length = notes_at_prefix("cde8").unwrap();
    assert_eq!(plain.note_index, with_length.note_index);
    assert_eq!(plain.pitches, with_length.pitches);
}

#[test]
fn command_only_prefix_has_no_note() {
    assert_eq!(notes_at_prefix("l8"), None);
    assert_eq!(notes_at_prefix("o4"), None);
}

#[test]
fn rest_does_not_become_a_sounding_note() {
    let after_rest = notes_at_prefix("cr").unwrap();
    assert_eq!(after_rest.pitches, vec![60]);
    assert_eq!(after_rest.note_index, 0);
}

#[test]
fn chord_sounds_every_member() {
    assert_eq!(pitches("'ceg'"), Some(vec![60, 64, 67]));
}

#[test]
fn note_index_distinguishes_repeated_pitches() {
    let first = notes_at_prefix("c").unwrap();
    let second = notes_at_prefix("cc").unwrap();
    assert_eq!(first.pitches, second.pitches);
    assert_ne!(first, second);
}

#[test]
fn velocity_command_changes_the_sounding_velocity() {
    assert_eq!(notes_at_prefix("c").unwrap().velocity, 127);
    assert!(notes_at_prefix("v8c").unwrap().velocity < 127);
}

#[test]
fn prefix_byte_index_counts_characters() {
    assert_eq!(prefix_byte_index("cde", 2), 2);
    assert_eq!(prefix_byte_index("cde", 9), 3);
}
