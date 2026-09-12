//! 実サーバーへの送信（state load と note on）と、掛かった実時間の内訳。
//!
//! どちらも「鳴らす小節の 1 つ前の小節」で起きる。その小節の WAV をスロットへ載せ
//! （[`prepare_measure_cues`]）、そのうえで note on を timeline へ積む（[`send_measure_note_on`]）。
//! 逆にすると、まだ載っていないスロットを指す note on が先に鳴って 1 つ前の同じ剰余の小節が出る。
//! 小節境界では 1 バイトも送らない。境界で送っていたころの無音とジッタは `docs/adr/0016`。

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

use cmrt_core::cache_wav::cache_wav_patch_with_slot;
use cmrt_realtime_play::RealtimePlayServerSupervisor;

use super::cues::{note_on_events, LiveCacheCue, MeasureLiveCues};
use super::measure_log::join_or_dash;
use super::timeline::MeasureTimeline;

/// 1 小節ぶんの送信に掛かった実時間の内訳と、その小節の発音位置。
///
/// `preload_next` / `note_on_next` は小節の途中で起きるので音には出ない（小節長を超えたら
/// 先読みが破綻する）。`prepare` / `note_on` は小節境界で演奏スレッドが止まっていた時間で、
/// 予約が当たっていれば両方 0。混ぜて 1 つの数字にすると「先読みが効いているのか、
/// 単に state load が軽かっただけなのか」が読めなくなる。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct MeasureSendTiming {
    /// この小節の WAV が、1 つ前の小節の先読みで既に載っていたか。`false` になるのは
    /// 演奏開始の 1 小節目と、先読みした小節と実際に進んだ小節が食い違ったとき
    /// （演奏中の AB リピート変更・小節数変更）だけ。
    pub(crate) preloaded: bool,
    /// この小節を鳴らす位置（timeline 原点からのフレーム数）。ジッタの判定材料はここ。
    /// 隣り合う小節でこの差が小節長ちょうどなら、サーバーが実際に鳴らすサンプル位置の差も
    /// 小節長ちょうどになる（フレームから作った秒は `round()` で必ず元のフレームへ戻る）。
    pub(crate) at_frames: u64,
    /// 小節境界で state load に費やした実時間。先読みが当たっていれば 0。
    pub(crate) prepare: Duration,
    /// 小節境界で note on の送信に費やした実時間。予約が当たっていれば 0。
    pub(crate) note_on: Duration,
    /// この小節を鳴らしている最中に、次の小節をスロットへ載せるのに掛かった実時間。
    /// 小節長に対する占有率がそのまま先読みの重さ。
    pub(crate) preload_next: Duration,
    /// 次の小節の note on を timeline へ積むのに掛かった実時間。全 track を 1 バッチで
    /// 投げるので track 数に比例せず、跳ねたら「1 件ずつ送る形」へ戻った合図。
    pub(crate) note_on_next: Duration,
}

/// 先読みで 1 スロットへ載せ終えた 1 小節ぶん。
///
/// `prepared` は state load が成功した cue だけ。失敗した行を note on の対象から外すのは、
/// 音源が載っていないところを鳴らすと**別の小節の音が出てしまう**ため
/// （スロットには 1 つ前の同じ剰余の小節が残っている）。
pub(crate) struct PreloadedMeasure {
    pub(crate) measure_index: usize,
    pub(crate) slot: usize,
    pub(crate) cues: MeasureLiveCues,
    pub(crate) prepared: Vec<LiveCacheCue>,
    pub(crate) elapsed: Duration,
}

/// 1 小節ぶんのキャッシュ WAV を、その小節のスロットへ載せる。
///
/// `prepare_live_patch` は応答待ちでブロックするので、鳴らす小節の 1 つ前の小節の中で呼ぶこと。
/// `on_prepared` には「何本ぶん済んだか」を 1 本ごとに渡す。演奏開始の 1 小節目はこれが
/// 「音が鳴るまで」overlay の進捗になり、1 本目だけ極端に重いので、終わってからまとめて
/// 数えたのでは進んで見えない。
pub(crate) fn prepare_measure_cues(
    play_server: &RealtimePlayServerSupervisor,
    measure_index: usize,
    slot: usize,
    cues: MeasureLiveCues,
    log_lines: &Arc<Mutex<VecDeque<String>>>,
    on_prepared: &mut dyn FnMut(usize),
) -> PreloadedMeasure {
    let started = Instant::now();
    let mut prepared: Vec<LiveCacheCue> = Vec::with_capacity(cues.cues.len());
    for (index, cue) in cues.cues.iter().enumerate() {
        // `.wav` で終わる patch 文字列でサーバーは cache-player を選び、`slot=N;` プレフィクスが
        // 載せるスロットを指定する（綴りの単一ソースは play server 側 `core-lib/src/cache_wav.rs`）。
        // state に入るのはパスであってファイルの中身ではない。
        let patch = cache_wav_patch_with_slot(slot, &cue.wav.to_string_lossy());
        if let Err(error) = play_server.prepare_live_patch(cue.instance, Some(&patch)) {
            crate::append_log_line(
                log_lines,
                format!(
                    "live-cache: prepare failed meas{} slot={slot} row={} instance={} \
                     error=\"{error:#}\"",
                    measure_index + 1,
                    cue.row,
                    cue.instance
                ),
            );
            on_prepared(index + 1);
            continue;
        }
        prepared.push(cue.clone());
        on_prepared(index + 1);
    }
    PreloadedMeasure {
        measure_index,
        slot,
        cues,
        prepared,
        elapsed: started.elapsed(),
    }
}

/// 載せ終えている全 track の note on を、その小節の発音位置 `at`（timeline 原点からのフレーム数。
/// 必ず [`MeasureTimeline`] から取る）へ timeline で予約する。戻り値は送信に掛かった実時間。
///
/// ここで state load を出してはいけない。出すとその応答待ちのぶんだけ予約が遅れ、
/// しかも 1 track ずつ順に返ってくるので track ごとにバラバラの時刻で音が切り替わる。
pub(crate) fn send_measure_note_on(
    play_server: &RealtimePlayServerSupervisor,
    timeline: &MeasureTimeline,
    measure: &PreloadedMeasure,
    at: u64,
    log_lines: &Arc<Mutex<VecDeque<String>>>,
) -> Duration {
    let started = Instant::now();
    let events = note_on_events(
        &measure.prepared,
        measure.slot,
        timeline.id(),
        timeline.seconds_of(at),
    );
    // 空のバッチはサーバーが `InvalidPayload` で弾く（1..=128 件しか受けない）。
    // 「鳴らすものが無い小節」は正常な状態なので、送らずに黙って抜ける。
    if !events.is_empty() {
        if let Err(error) = play_server.send_timeline_events(&events) {
            let rows = join_or_dash(measure.prepared.iter().map(|cue| format!("row{}", cue.row)));
            crate::append_log_line(
                log_lines,
                format!(
                    "live-cache: note on failed meas{} rows={rows} at_frames={at} error=\"{error:#}\"",
                    measure.measure_index + 1,
                ),
            );
        }
    }
    started.elapsed()
}
