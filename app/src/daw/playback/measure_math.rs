/// 現在の再生カーソルから、実際に再生すべき小節 index を求める。
///
/// `current_measure_index` が `effective_count` 以上なら、ループ先頭の `0` に巻き戻す。
pub(in crate::daw) fn current_play_measure_index(
    current_measure_index: usize,
    effective_count: usize,
) -> usize {
    if current_measure_index < effective_count {
        current_measure_index
    } else {
        0
    }
}

/// 現在小節の次に先読みすべき小節 index を求める。
///
/// `effective_count` を法として 1 つ進め、末尾なら先頭へ折り返す。
pub(in crate::daw) fn following_measure_index(
    current_measure_index: usize,
    effective_count: usize,
) -> usize {
    (current_measure_index + 1) % effective_count
}

pub(in crate::daw::playback) fn format_playback_measure_resolution_log(
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

pub(in crate::daw::playback) fn format_playback_measure_advance_log(
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
