use super::{label_columns, swing_label};
use crate::ui::layout::LABEL_WIDTH;

/// 書式文字列と [`LABEL_WIDTH`] はコンパイラが整合を見ない。ここがずれると描画は
/// 無事なのに mouse hit test だけ列がずれる、という追いにくい壊れ方をする。
#[test]
fn the_label_columns_are_exactly_as_wide_as_the_layout_says() {
    let header = label_columns("#", "V", "S", "PATCH", "GAIN", "NOTE", "SW");
    assert_eq!(header.chars().count(), usize::from(LABEL_WIDTH));

    // 各欄が最大幅まで埋まった行。patch は `truncate_patch` が 24 桁へ収めてある。
    let full = label_columns("16", "4", "S", &"x".repeat(24), "-12.0", "127", "66");
    assert_eq!(full.chars().count(), usize::from(LABEL_WIDTH));

    // 先頭行以外は patch / gain / swing が空になる。
    let empty = label_columns("", "", "", "", "", "", "");
    assert_eq!(empty.chars().count(), usize::from(LABEL_WIDTH));
}

#[test]
fn the_header_carries_the_swing_column_after_note() {
    let header = label_columns("#", "V", "S", "PATCH", "GAIN", "NOTE", "SW");
    assert!(header.ends_with("NOTE SW "), "{header:?}");
}

#[test]
fn a_row_without_offbeat_attacks_shows_a_dash_instead_of_a_number() {
    let swings = vec![Some(66), None];
    assert_eq!(swing_label(&swings, 0), "66");
    assert_eq!(swing_label(&swings, 1), "-");
    // 行が範囲外（描画と state がずれた瞬間）でも数字を捏造しない。
    assert_eq!(swing_label(&swings, 9), "-");
}
