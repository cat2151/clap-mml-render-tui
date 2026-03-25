pub(crate) fn current_play_measure_index(
    current_measure_index: usize,
    effective_count: usize,
) -> usize {
    if current_measure_index < effective_count {
        current_measure_index
    } else {
        0
    }
}

pub(crate) fn following_measure_index(
    current_measure_index: usize,
    effective_count: usize,
) -> usize {
    (current_measure_index + 1) % effective_count
}

pub(crate) fn format_playback_measure_resolution_log(
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

pub(crate) fn format_playback_measure_advance_log(
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
