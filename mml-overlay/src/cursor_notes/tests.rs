use super::*;

fn pitches(prefix: &str) -> Option<Vec<u8>> {
    notes_at_prefix(prefix).map(|notes| notes.pitches)
}

fn duration(prefix: &str) -> Duration {
    notes_at_prefix(prefix).expect("prefix has a note").duration
}

fn cursor_pitches(line: &str, cursor: usize) -> Option<Vec<u8>> {
    notes_at_cursor(line, cursor).map(|notes| notes.pitches)
}

#[test]
fn empty_input_has_no_note() {
    assert_eq!(notes_at_prefix(""), None);
    assert_eq!(notes_at_cursor("", 0), None);
}

#[test]
fn bare_note_sounds_middle_c() {
    assert_eq!(pitches("c"), Some(vec![60]));
}

#[test]
fn typing_each_letter_advances_the_note() {
    assert_eq!(cursor_pitches("c", 1), Some(vec![60]));
    assert_eq!(cursor_pitches("cd", 2), Some(vec![62]));
    assert_eq!(cursor_pitches("cde", 3), Some(vec![64]));
}

#[test]
fn moving_the_cursor_left_returns_to_the_earlier_note() {
    assert_eq!(cursor_pitches("cde", 3), Some(vec![64]));
    assert_eq!(cursor_pitches("cde", 2), Some(vec![62]));
    assert_eq!(cursor_pitches("cde", 1), Some(vec![60]));
    // 行頭では左に単位が無いので、右隣の `c` を鳴らす。
    assert_eq!(cursor_pitches("cde", 0), Some(vec![60]));
}

#[test]
fn modifier_shifts_the_pitch() {
    assert_eq!(pitches("c+"), Some(vec![61]));
    assert_eq!(pitches("c-"), Some(vec![59]));
}

#[test]
fn octave_and_modifier_accumulate_before_the_cursor() {
    // `<` はオクターブ上げ。カーソルのある単位より前の状態がそのまま効く。
    assert_eq!(cursor_pitches("c+1<c+", 6), Some(vec![73]));
}

#[test]
fn command_only_input_has_no_note() {
    assert_eq!(notes_at_prefix("l8"), None);
    assert_eq!(notes_at_cursor("l8", 2), None);
    assert_eq!(notes_at_cursor("o4", 2), None);
}

/// 休符の上では鳴らすものが無い。「直前の音が鳴りっぱなしで休符が書けない」を避ける。
#[test]
fn a_rest_sounds_nothing() {
    assert_eq!(notes_at_cursor("cr", 2), None);
    assert_eq!(notes_at_cursor("cr4", 3), None);
}

#[test]
fn chord_sounds_every_member() {
    assert_eq!(pitches("'ceg'"), Some(vec![60, 64, 67]));
}

#[test]
fn velocity_command_changes_the_sounding_velocity() {
    assert_eq!(notes_at_prefix("c").unwrap().velocity, 127);
    assert!(notes_at_prefix("v8c").unwrap().velocity < 127);
}

// --- 発音単位 ---

/// カーソルが単位の途中にあっても、単位ぜんぶが鳴る。カーソルの手前で切ると
/// `c4` の途中では `4` が無かったことになり、書いた音長を耳で確かめられない。
#[test]
fn the_cursor_inside_a_note_sounds_the_whole_note() {
    assert_eq!(duration("c4"), Duration::from_millis(500));
    assert_eq!(
        notes_at_cursor("c4", 1).unwrap().duration,
        Duration::from_millis(500)
    );
}

/// 和音の中はどこにカーソルがあっても同じ 1 単位。閉じ `'` の上でも構成音が揃う。
#[test]
fn the_cursor_anywhere_in_a_chord_sounds_the_whole_chord() {
    for cursor in 0..=5 {
        assert_eq!(cursor_pitches("'ceg'", cursor), Some(vec![60, 64, 67]));
    }
}

/// 同じ単位に留まるかどうかは範囲で分かる。値だけでは `'ceg` と `'ceg'` を
/// 区別できず、閉じクォートの上だけ無音になっていた。
#[test]
fn the_span_tells_units_apart_when_the_notes_are_identical() {
    let open = notes_at_cursor("'ceg", 4).unwrap();
    let closed = notes_at_cursor("'ceg'", 5).unwrap();

    assert_eq!(open.pitches, closed.pitches);
    assert_eq!(open.span, 0..4);
    assert_eq!(closed.span, 0..5);
    assert_ne!(open, closed);
}

/// 同じ音高を並べても単位が違う。範囲が違うので鳴らし直せる。
#[test]
fn repeated_pitches_are_different_units() {
    let first = notes_at_cursor("cc", 1).unwrap();
    let second = notes_at_cursor("cc", 2).unwrap();

    assert_eq!(first.pitches, second.pitches);
    assert_ne!(first.span, second.span);
}

/// カーソルが動かなければ同じ単位のまま。
#[test]
fn staying_in_the_same_unit_gives_the_same_notes() {
    assert_eq!(notes_at_cursor("c4.", 1), notes_at_cursor("c4.", 3));
}

// --- コード表記 ---

/// コード表記は、打ち終わった時点で構成音ぜんぶが鳴る。
/// 行を移るまで待たされると、書いている最中はずっと単音しか聞こえない。
#[test]
fn a_chord_name_sounds_every_member_while_typing() {
    let notes = notes_at_cursor("C", 1).unwrap();

    assert_eq!(notes.pitches, vec![60, 64, 67]);
    assert!(notes.from_chord);
}

/// `II` の途中でも、カーソル左側の `I` ではなく chord 全体を鳴らす。
#[test]
fn every_position_in_a_roman_chord_uses_its_full_span_and_pitches() {
    let first_character = notes_at_cursor("II", 1).unwrap();
    let end = notes_at_cursor("II", 2).unwrap();
    let tonic = notes_at_cursor("I", 1).unwrap();

    assert_eq!(first_character.span, 0..2);
    assert_eq!(end.span, 0..2);
    assert_eq!(first_character.pitches, vec![62, 66, 69]);
    assert_eq!(first_character, end);
    assert_eq!(tonic.pitches, vec![60, 64, 67]);
    assert_ne!(first_character.pitches, tonic.pitches);
    assert!(first_character.from_chord);
}

#[test]
fn a_progression_uses_the_pitches_of_the_selected_source_chord() {
    let first = notes_at_cursor("I II", 1).unwrap();
    let second = notes_at_cursor("I II", 3).unwrap();

    assert_eq!(first.span, 0..1);
    assert_eq!(first.pitches, vec![60, 64, 67]);
    assert_eq!(second.span, 2..4);
    assert_eq!(second.pitches, vec![62, 66, 69]);
    assert!(first.from_chord);
    assert!(second.from_chord);
}

/// 小文字の MML はコード表記と紛れない。単音のまま鳴る。
#[test]
fn lowercase_mml_is_not_read_as_a_chord() {
    let notes = notes_at_cursor("c", 1).unwrap();

    assert_eq!(notes.pitches, vec![60]);
    assert!(!notes.from_chord);
}

/// コード進行を書いている途中は、カーソル直前のコードが鳴る。
#[test]
fn typing_a_progression_sounds_the_latest_chord() {
    let notes = notes_at_cursor("C Am", 4).unwrap();

    assert_eq!(notes.pitches, vec![69, 72, 76]);
    assert!(notes.from_chord);
}

/// コード表記として読めない行は、これまでどおり MML として解釈する。
#[test]
fn a_line_that_is_not_a_chord_falls_back_to_mml() {
    let notes = notes_at_cursor("o5c+", 4).unwrap();

    assert!(!notes.from_chord);
    assert_eq!(notes.pitches, vec![61]);
}

// --- 音長 ---

/// 音長を書いていないときは本家の既定 `l8`。既定テンポ 120 なら 250ms。
#[test]
fn a_bare_note_lasts_the_default_length() {
    assert_eq!(duration("c"), Duration::from_millis(250));
}

/// 書いた音長がそのまま鳴る長さになる。固定長で切ると `c1` の `1` が効かない。
#[test]
fn a_written_note_length_becomes_the_sounding_length() {
    assert_eq!(duration("c1"), Duration::from_millis(2000));
    assert_eq!(duration("c4"), Duration::from_millis(500));
    assert_eq!(duration("c16"), Duration::from_millis(125));
}

/// 付点も本家の解釈どおり。4 分音符の 1.5 倍。
#[test]
fn a_dot_extends_the_sounding_length() {
    assert_eq!(duration("c4."), Duration::from_millis(750));
}

/// `l` の既定音長はカーソルより前にあれば効く。
#[test]
fn the_default_length_command_changes_the_sounding_length() {
    assert_eq!(duration("l4c"), Duration::from_millis(500));
}

/// テンポも効く。`t60` なら 8 分音符は 500ms。
#[test]
fn the_tempo_command_changes_the_sounding_length() {
    assert_eq!(duration("t60c"), Duration::from_millis(500));
}

#[test]
fn cursor_byte_index_counts_characters() {
    assert_eq!(cursor_byte_index("cde", 2), 2);
    assert_eq!(cursor_byte_index("cde", 9), 3);
}
