//! 小節 1 つを 1 行にまとめるログ。

use super::cues::{measure_slot, MeasureLiveCues};
use super::send::MeasureSendTiming;

/// 小節ごとの送信内容を 1 行にまとめる。
///
/// 例（実際は 1 行）:
///
/// ```text
/// meas3: live-cache slot=0 preload=hit sent=row2/i0,row4/i2 silent=row3 over_limit=-
///   at_frames=240000 prepare_ms=0.0 note_on_ms=0.0 next=meas4/slot1 next_ms=105.3
///   next_note_on_ms=0.2
/// ```
///
/// 読み方:
///
/// - `slot` / `next` … その小節の WAV を載せたスロットと、この小節の**最中に**先読みした小節
/// - `preload=hit|miss` … 小節境界に到達した時点で WAV が載っていて note on も
///   予約済みだったか。`miss` は演奏開始の 1 小節目か、演奏中に AB リピート・
///   小節数が変わったとき
/// - `at_frames` … **この小節を鳴らす位置**（timeline 原点からのフレーム数）。
///   サーバーログの `cmrt-live: event=apply-midi ... clock=` と同じ単位で、
///   隣り合う小節でこの差が小節長ちょうどなら発音位置のジッタは 0
/// - `prepare_ms` … **小節境界で** state load に止まっていた時間。`hit` なら 0.0
/// - `note_on_ms` … **小節境界で** note on の送信に止まっていた時間。`hit` なら 0.0
///   （予約は 1 つ前の小節で済ませてある）
/// - `next_ms` … 次の小節の先読みに掛かった時間。小節の途中なので音には出ないが、
///   小節長を超えたら先読みが破綻する
/// - `next_note_on_ms` … 次の小節の note on を timeline へ積むのに掛かった時間。
///   track 数に比例したら「まとめて 1 コマンド」が壊れた合図
pub(crate) fn format_live_cache_measure_log(
    measure_index: usize,
    next_measure_index: usize,
    cues: &MeasureLiveCues,
    timing: MeasureSendTiming,
) -> String {
    let sent = join_or_dash(
        cues.cues
            .iter()
            .map(|cue| format!("row{}/i{}", cue.row, cue.instance)),
    );
    let silent = join_or_dash(cues.silent_rows.iter().map(|row| format!("row{row}")));
    let over_limit = join_or_dash(
        cues.rows_over_instance_limit
            .iter()
            .map(|row| format!("row{row}")),
    );
    let preload = if timing.preloaded { "hit" } else { "miss" };
    format!(
        "meas{}: live-cache slot={} preload={preload} sent={sent} silent={silent} \
         over_limit={over_limit} at_frames={} prepare_ms={:.1} note_on_ms={:.1} \
         next=meas{}/slot{} next_ms={:.1} next_note_on_ms={:.1}",
        measure_index + 1,
        measure_slot(measure_index),
        timing.at_frames,
        timing.prepare.as_secs_f64() * 1_000.0,
        timing.note_on.as_secs_f64() * 1_000.0,
        next_measure_index + 1,
        measure_slot(next_measure_index),
        timing.preload_next.as_secs_f64() * 1_000.0,
        timing.note_on_next.as_secs_f64() * 1_000.0,
    )
}

pub(crate) fn join_or_dash(items: impl Iterator<Item = String>) -> String {
    let joined = items.collect::<Vec<_>>().join(",");
    if joined.is_empty() {
        "-".to_string()
    } else {
        joined
    }
}
