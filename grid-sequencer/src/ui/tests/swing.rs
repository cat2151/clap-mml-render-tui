//! NOTE grid の SWING 欄。instance ごとの shuffle 量を行ごとに見せる表示専用の列。

use super::*;
use crate::SWING_MAX;

/// SWING 欄の左端。中央寄せで grid の左端が動くので、列は直書きせず layout から引く。
fn swing_column(screen: &GridSequencerScreen) -> usize {
    usize::from(test_layout(screen).swing_column())
}

/// 見出しと値が同じ桁に並ぶこと。値だけ出ていても何の数字か分からない。
#[test]
fn the_swing_column_shows_the_shuffle_amount_of_each_instance() {
    // 行1は裏拍（step 1）に、行2は表拍（step 2）に note on を置く。
    let mut screen = screen_with_first_row(60, &[1]);
    screen.state.rows_mut()[0].swing = SWING_MAX;
    let second = &mut screen.state.rows_mut()[1];
    second.swing = SWING_MAX;
    second.pattern.draw_span(2, 2);

    let rendered = render(&screen);
    let lines = rendered.lines().collect::<Vec<_>>();
    let column = swing_column(&screen);

    assert_eq!(slice_chars(lines[FIRST_ROW_Y - 1], column, 2), "SW");
    assert_eq!(slice_chars(lines[FIRST_ROW_Y], column, 2), "66");
    // 表拍にしか note on が無い行は、値を持っていても跳ねないので `-`。
    assert_eq!(slice_chars(lines[FIRST_ROW_Y + 1], column, 2), " -");
}

/// 跳ねる行でも 50 なら 50 と出す。「対象外」と「対象だが等分」は別物。
#[test]
fn an_offbeat_row_left_at_fifty_shows_the_number_not_a_dash() {
    let screen = screen_with_first_row(60, &[3]);

    let rendered = render(&screen);
    let lines = rendered.lines().collect::<Vec<_>>();

    assert_eq!(
        slice_chars(lines[FIRST_ROW_Y], swing_column(&screen), 2),
        "50"
    );
}

/// 何も鳴らさない行は跳ねようがない。全行が `-` で始まる。
#[test]
fn a_silent_row_shows_a_dash() {
    let mut screen = GridSequencerScreen::new(None);
    screen.state.rows_mut()[0].swing = SWING_MAX;

    let rendered = render(&screen);
    let lines = rendered.lines().collect::<Vec<_>>();

    assert_eq!(
        slice_chars(lines[FIRST_ROW_Y], swing_column(&screen), 2),
        " -"
    );
}
