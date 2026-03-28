use std::time::{Duration, Instant};

/// 現在の再生カーソルから、実際に再生すべき小節 index を求める。
///
/// `current_measure_index` が `effective_count` 以上なら、ループ先頭の `0` に巻き戻す。
pub(in crate::daw) fn current_play_measure_index(
    current_measure_index: usize,
    effective_count: usize,
    ab_repeat_range: Option<(usize, usize)>,
) -> usize {
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
/// `effective_count` を法として 1 つ進め、末尾なら先頭へ折り返す。
pub(in crate::daw) fn following_measure_index(
    current_measure_index: usize,
    effective_count: usize,
    ab_repeat_range: Option<(usize, usize)>,
) -> usize {
    let (loop_start_measure_index, loop_end_measure_index) =
        ab_repeat_range.unwrap_or((0, effective_count - 1));
    if current_measure_index >= loop_end_measure_index {
        loop_start_measure_index
    } else {
        current_measure_index + 1
    }
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

/// 次小節の開始前に append margin を確保できる最遅時刻を返す。
///
/// append margin を確保できる最遅時刻が現在小節の開始時刻を超えない場合は、
/// 現在小節の開始時刻までにクランプする。
pub(in crate::daw::playback) fn future_chunk_append_deadline(
    measure_start: Instant,
    measure_duration: Duration,
    append_margin: Duration,
) -> Instant {
    let next_measure_start = measure_start + measure_duration;
    match next_measure_start.checked_sub(append_margin) {
        // append margin を確保できる最遅時刻が現在小節の開始時刻を超えない場合
        // （例: measure_duration < append_margin や Instant の表現可能範囲外へのアンダーフロー）
        // は、measure_start までにクランプしてフォールバックする。
        Some(deadline) if deadline > measure_start => deadline,
        _ => measure_start,
    }
}

/// future chunk の append 実績を、次小節開始に対する lead/late 付きでログ文字列にする。
///
/// 例: `play: queue meas3 append lead=48ms (target_margin=50ms)`
pub(in crate::daw::playback) fn format_playback_future_append_log(
    measure_index: usize,
    append_time: Instant,
    measure_start: Instant,
    target_margin: Duration,
) -> String {
    let target_margin_ms = target_margin.as_millis();
    if let Some(lead) = measure_start.checked_duration_since(append_time) {
        format!(
            "play: queue meas{} append lead={}ms (target_margin={}ms)",
            measure_index + 1,
            lead.as_millis(),
            target_margin_ms,
        )
    } else {
        format!(
            "play: queue meas{} append late={}ms (target_margin={}ms)",
            measure_index + 1,
            append_time.duration_since(measure_start).as_millis(),
            target_margin_ms,
        )
    }
}

/// 次小節の実再生開始時刻を返す。
///
/// append が予定どおり境界前に終わった場合は `expected_measure_start` を維持し、
/// append が境界を過ぎて遅れた場合は append 完了時刻へ再同期する。
pub(in crate::daw::playback) fn resolved_measure_start_after_append(
    expected_measure_start: Instant,
    append_time: Instant,
) -> Instant {
    std::cmp::max(expected_measure_start, append_time)
}
