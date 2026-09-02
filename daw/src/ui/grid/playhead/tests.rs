use super::super::MEASURE_CELL_WIDTH;
use super::*;

const WIDTH: usize = MEASURE_CELL_WIDTH;

/// 4/4・セル 4 桁は 1 拍 1 桁の 4 段階で進む。
#[test]
fn four_four_fills_one_column_per_beat() {
    // 1 小節 = 4 秒 → 1 拍 = 1 秒。
    assert_eq!(filled_columns(0.0, 4.0, 4, WIDTH), 1);
    assert_eq!(filled_columns(0.9, 4.0, 4, WIDTH), 1);
    assert_eq!(filled_columns(1.0, 4.0, 4, WIDTH), 2);
    assert_eq!(filled_columns(2.5, 4.0, 4, WIDTH), 3);
    assert_eq!(filled_columns(3.9, 4.0, 4, WIDTH), 4);
}

/// 小節末尾を過ぎても最終拍に留まる（次小節へは playback 側が進める）。
#[test]
fn overrun_stays_on_the_last_beat() {
    assert_eq!(filled_columns(4.0, 4.0, 4, WIDTH), 4);
    assert_eq!(filled_columns(99.0, 4.0, 4, WIDTH), 4);
}

/// 演奏が始まった瞬間から必ず 1 桁は塗る（現在小節が消えて見えないように）。
#[test]
fn start_of_measure_is_never_empty() {
    assert_eq!(filled_columns(0.0, 4.0, 7, WIDTH), 1);
    assert_eq!(filled_columns(0.0, 4.0, 16, WIDTH), 1);
}

/// 拍子とセル桁数が割り切れなくても、その拍を含む桁まで塗って溢れない。
#[test]
fn odd_beat_counts_stay_within_the_cell() {
    // 3/4: 1 拍目 → 2 桁、2 拍目 → 3 桁、3 拍目 → 4 桁。
    assert_eq!(filled_columns(0.0, 3.0, 3, WIDTH), 2);
    assert_eq!(filled_columns(1.0, 3.0, 3, WIDTH), 3);
    assert_eq!(filled_columns(2.0, 3.0, 3, WIDTH), 4);
    for beat_count in 1..=32 {
        for step in 0..=20 {
            let elapsed = 3.0 * f64::from(step) / 20.0;
            let filled = filled_columns(elapsed, 3.0, beat_count, WIDTH);
            assert!(
                (1..=WIDTH).contains(&filled),
                "beat_count={beat_count} elapsed={elapsed} filled={filled}"
            );
        }
    }
}

/// 小節長が 0 / 異常値でも panic せず、全桁塗りへ倒す。
#[test]
fn degenerate_duration_fills_the_whole_cell() {
    assert_eq!(filled_columns(1.0, 0.0, 4, WIDTH), WIDTH);
    assert_eq!(filled_columns(f64::NAN, 4.0, 4, WIDTH), WIDTH);
    // 拍子 0 は 1 拍として扱う → 小節まるごと 1 拍ぶん = 全桁。
    assert_eq!(filled_columns(1.0, 4.0, 0, WIDTH), WIDTH);
}

/// `M3` は `>3` へ。A-B マーカーは情報を落とさずそのまま残す。
#[test]
fn header_label_replaces_only_the_measure_prefix() {
    assert_eq!(header_label("M3"), ">3");
    assert_eq!(header_label("M100"), ">100");
    assert_eq!(header_label("A3"), "A3");
    assert_eq!(header_label("B7"), "B7");
    assert_eq!(header_label("AB3"), "AB3");
}

fn spans_of(label: &str, filled_columns: usize, state: DawPlayState) -> Vec<Span<'static>> {
    header_spans(
        label,
        1,
        Playhead {
            measure_index: 0,
            filled_columns,
            state,
        },
        Style::default().fg(MONOKAI_YELLOW),
    )
}

/// 列幅（セル 4 桁 + 区切り 1 桁）は playhead が出ても変わらない。
#[test]
fn header_spans_keep_the_column_width() {
    for filled_columns in 0..=4 {
        let spans = spans_of(">3", filled_columns, DawPlayState::Playing);
        let width: usize = spans.iter().map(|span| span.content.chars().count()).sum();
        assert_eq!(
            width,
            MEASURE_CELL_WIDTH + COLUMN_GAP,
            "filled={filled_columns}"
        );
    }
}

/// 桁あふれするラベルはセル内で切る（隣の列をずらさない）。
#[test]
fn header_spans_truncate_overlong_labels() {
    let spans = spans_of(">10000", 4, DawPlayState::Playing);
    let text: String = spans.iter().map(|span| span.content.as_ref()).collect();
    assert_eq!(text, ">100 ");
}

/// 塗った桁は反転、残りは暗い背景、区切り空白は塗らない。
#[test]
fn header_spans_paint_only_the_elapsed_columns() {
    let spans = spans_of(">3", 2, DawPlayState::Playing);

    assert_eq!(spans[0].content.as_ref(), ">3");
    assert_eq!(spans[0].style.bg, Some(MONOKAI_YELLOW));
    assert_eq!(spans[0].style.fg, Some(MONOKAI_BG));
    assert_eq!(spans[1].content.as_ref(), "  ");
    assert_eq!(spans[1].style.bg, Some(MONOKAI_CURSOR_BG));
    assert_eq!(spans[2].content.as_ref(), " ");
    assert_eq!(spans[2].style.bg, None);
}

/// preview は色だけが変わる。
#[test]
fn preview_uses_its_own_fill_color() {
    let spans = spans_of(">3", 2, DawPlayState::Preview);
    assert_eq!(spans[0].style.bg, Some(MONOKAI_PURPLE));
}

/// 残りの桁は A-B マーカーの前景色を保つ（演奏位置は背景だけで表す）。
#[test]
fn unfilled_columns_keep_the_ab_marker_color() {
    let spans = spans_of("A3", 1, DawPlayState::Playing);
    assert_eq!(spans[1].style.fg, Some(MONOKAI_YELLOW));
}
