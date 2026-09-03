/// 現在の再生カーソルから、実際に再生すべき小節 index を求める。
///
/// `current_measure_index` が現在のループ範囲外なら、そのループ先頭へ巻き戻す。
/// `effective_count == 0` は呼び出し側で避けているが、安全のため 0 を返す。
pub(crate) fn current_play_measure_index(
    current_measure_index: usize,
    effective_count: usize,
    ab_repeat_range: Option<(usize, usize)>,
) -> usize {
    if effective_count == 0 {
        return 0;
    }
    let (loop_start_measure_index, loop_end_measure_index) =
        ab_repeat_range.unwrap_or((0, effective_count - 1));
    if (loop_start_measure_index..=loop_end_measure_index).contains(&current_measure_index) {
        current_measure_index
    } else {
        loop_start_measure_index
    }
}

/// 現在小節の次に先読みすべき小節 index を求める。
///
/// 現在のループ範囲内で 1 つ進め、末尾ならそのループ先頭へ折り返す。
/// `effective_count == 0` は呼び出し側で避けているが、安全のため 0 を返す。
pub(crate) fn following_measure_index(
    current_measure_index: usize,
    effective_count: usize,
    ab_repeat_range: Option<(usize, usize)>,
) -> usize {
    if effective_count == 0 {
        return 0;
    }
    let (loop_start_measure_index, loop_end_measure_index) =
        ab_repeat_range.unwrap_or((0, effective_count - 1));
    if current_measure_index >= loop_end_measure_index {
        loop_start_measure_index
    } else {
        current_measure_index + 1
    }
}

pub(in crate::playback) fn format_playback_measure_resolution_log(
    measure_index_cursor: usize,
    resolved_measure_index: usize,
    effective_count: usize,
) -> String {
    format!(
        "play: sync resolve cursor=meas{} -> current=meas{} (effective_count={effective_count})",
        measure_index_cursor + 1,
        resolved_measure_index + 1,
    )
}

pub(in crate::playback) fn format_playback_measure_advance_log(
    current_measure_index: usize,
    lookahead_measure_index: usize,
    effective_count: usize,
) -> String {
    format!(
        "play: sync advance current=meas{} -> next=meas{} (effective_count={effective_count})",
        current_measure_index + 1,
        lookahead_measure_index + 1,
    )
}
