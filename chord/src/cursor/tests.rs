use super::*;

#[test]
fn a_note_is_one_unit() {
    assert_eq!(cursor_sounding_unit("cde", 1), Some(0..1));
    assert_eq!(cursor_sounding_unit("cde", 2), Some(1..2));
    assert_eq!(cursor_sounding_unit("cde", 3), Some(2..3));
}

/// 音長も付点も同じ単位。`c4` の途中にカーソルがあっても 4 分音符のまま鳴らせる。
#[test]
fn a_note_unit_covers_its_length_and_dots() {
    assert_eq!(cursor_sounding_unit("c4.", 1), Some(0..3));
    assert_eq!(cursor_sounding_unit("c4.", 2), Some(0..3));
    assert_eq!(cursor_sounding_unit("c4.", 3), Some(0..3));
}

/// 和音は閉じクォートまでで 1 つ。中をカーソルが通っても単位は変わらない。
#[test]
fn a_chord_is_one_unit_including_both_quotes() {
    for cursor in 0..=5 {
        assert_eq!(cursor_sounding_unit("'ceg'", cursor), Some(0..5));
    }
}

/// 書きかけの和音も鳴らせる。閉じた瞬間に範囲が伸びる。
#[test]
fn an_unfinished_chord_is_a_shorter_unit() {
    assert_eq!(cursor_sounding_unit("'ceg", 4), Some(0..4));
    assert_eq!(cursor_sounding_unit("'ceg'", 5), Some(0..5));
}

#[test]
fn a_rest_has_nothing_to_sound() {
    assert_eq!(cursor_sounding_unit("cr4", 3), None);
    assert_eq!(cursor_sounding_unit("cr4", 2), None);
}

#[test]
fn a_command_has_nothing_to_sound() {
    assert_eq!(cursor_sounding_unit("o5", 2), None);
    assert_eq!(cursor_sounding_unit("l8", 2), None);
    assert_eq!(cursor_sounding_unit("c<", 2), None);
}

/// どの単位にも触れていない空白は鳴らすものが無い。
#[test]
fn whitespace_between_units_has_nothing_to_sound() {
    assert_eq!(cursor_sounding_unit("c  d", 2), None);
    assert_eq!(cursor_sounding_unit("", 0), None);
}

/// カーソルの左に単位が無ければ右隣の単位を見る。行頭で `c` が鳴るのはこのため。
#[test]
fn a_cursor_with_nothing_on_its_left_takes_the_unit_on_its_right() {
    assert_eq!(cursor_sounding_unit("cde", 0), Some(0..1));
    assert_eq!(cursor_sounding_unit("c d", 2), Some(2..3));
}

#[test]
fn every_caret_position_in_a_chord_selects_the_whole_chord() {
    for cursor in 0..=2 {
        assert_eq!(cursor_sounding_unit("II", cursor), Some(0..2));
    }
}

#[test]
fn chord_progressions_select_one_source_chord_at_a_time() {
    for (line, expected) in [
        (
            "I II",
            vec![Some(0..1), Some(0..1), Some(2..4), Some(2..4), Some(2..4)],
        ),
        (
            "I-II",
            vec![Some(0..1), Some(0..1), Some(2..4), Some(2..4), Some(2..4)],
        ),
    ] {
        let actual = (0..=line.len())
            .map(|cursor| cursor_sounding_unit(line, cursor))
            .collect::<Vec<_>>();
        assert_eq!(actual, expected, "line {line:?}");
    }
}

#[test]
fn chord_directives_and_whitespace_are_not_sounding_units() {
    let line = "Key:C   I";
    for cursor in 0..=7 {
        assert_eq!(cursor_sounding_unit(line, cursor), None, "cursor {cursor}");
    }
    assert_eq!(cursor_sounding_unit(line, 8), Some(8..9));
    assert_eq!(cursor_sounding_unit(line, 9), Some(8..9));

    let line = "drop2   I";
    for cursor in 0..=7 {
        assert_eq!(cursor_sounding_unit(line, cursor), None, "cursor {cursor}");
    }
    assert_eq!(cursor_sounding_unit(line, 8), Some(8..9));
    assert_eq!(cursor_sounding_unit(line, 9), Some(8..9));
}

#[test]
fn dialect_and_unicode_chords_use_original_input_ranges() {
    let line = "ii-V-I";
    for (cursor, expected) in [
        (0, 0..2),
        (1, 0..2),
        (2, 0..2),
        (3, 3..4),
        (4, 3..4),
        (5, 5..6),
        (6, 5..6),
    ] {
        assert_eq!(cursor_sounding_unit(line, cursor), Some(expected));
    }

    let line = "C♯・II";
    for (cursor, expected) in [(0, 0..4), (1, 0..4), (4, 0..4), (7, 7..9), (9, 7..9)] {
        assert_eq!(cursor_sounding_unit(line, cursor), Some(expected));
    }
}

/// 小文字の MML はコード表記と紛れない。発音単位のほうで切れる。
#[test]
fn a_lowercase_line_is_read_as_mml() {
    assert_eq!(cursor_sounding_unit("ceg", 3), Some(2..3));
}

/// ふつうの MML 行をコード表記と取り違えない。取り違えると発音単位が丸ごと効かなくなる。
#[test]
fn a_realistic_mml_line_takes_the_mml_path() {
    let line = "o5 l8 'ceg' r4 <d+4. c";

    assert_eq!(cursor_sounding_unit(line, 11), Some(6..11)); // 'ceg'
    assert_eq!(cursor_sounding_unit(line, 20), Some(16..20)); // d+4.
    assert_eq!(cursor_sounding_unit(line, 22), Some(21..22)); // 末尾の c
    assert_eq!(cursor_sounding_unit(line, 14), None); // r4
    assert_eq!(cursor_sounding_unit(line, 5), None); // l8
}

/// 行頭 patch JSON があっても、範囲は行のバイト位置のまま返る。
#[test]
fn a_leading_patch_json_does_not_shift_the_range() {
    let line = r#"{"Surge XT patch": "Leads/Lead 1.fxp"} cde"#;
    let mml_start = line.len() - 3;

    assert_eq!(
        cursor_sounding_unit(line, line.len()),
        Some(mml_start + 2..mml_start + 3)
    );
}

#[test]
fn half_written_input_does_not_panic() {
    for line in ["'ce", "c+", "o", "kt-", "@", "'", ";", "xyz", "C Am7/"] {
        for cursor in 0..=line.len() {
            cursor_sounding_unit(line, cursor);
        }
    }
}
