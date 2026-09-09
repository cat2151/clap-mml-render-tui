use crate::ui::text::{display_width, fit_width, right_align};

#[test]
fn a_text_shorter_than_the_column_is_padded_to_the_column_width() {
    assert_eq!(fit_width("I-V", 6), "I-V   ");
    assert_eq!(display_width(&fit_width("I-V", 6)), 6);
}

#[test]
fn a_text_longer_than_the_column_is_cut_with_an_ellipsis() {
    assert_eq!(fit_width("I-V-VIm-IV", 6), "I-V-V…");
    assert_eq!(display_width(&fit_width("I-V-VIm-IV", 6)), 6);
}

/// 全角は 2 桁として数える。文字数で数えると section 名に日本語が入った瞬間に列がずれる。
#[test]
fn full_width_characters_count_as_two_columns() {
    assert_eq!(display_width("サビ"), 4);
    assert_eq!(display_width(&fit_width("サビ", 8)), 8);
    assert_eq!(display_width(&fit_width("サビサビサビ", 7)), 7);
}

#[test]
fn a_zero_width_column_draws_nothing() {
    assert_eq!(fit_width("I-V", 0), "");
}

/// 行番号は右詰め。桁が増えても列がずれない。
#[test]
fn row_numbers_are_right_aligned_in_their_column() {
    assert_eq!(right_align("4", 2), " 4");
    assert_eq!(
        display_width(&right_align("128", 2)),
        3,
        "溢れたら削らずにはみ出す"
    );
}
