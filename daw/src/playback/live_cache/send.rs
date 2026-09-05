//! 実サーバーへの送信（state load と note on）と、掛かった実時間の内訳。
//!
//! **どちらも「鳴らす小節の 1 つ前の小節」で起きる。** その小節の WAV をスロットへ
//! 載せ（[`prepare_measure_cues`]）、そのうえで note on を timeline へ積む
//! （[`send_measure_note_on`]）。順序は必ずこの向き。逆にすると、まだ載っていない
//! スロットを指す note on が先に鳴って**1 つ前の同じ剰余の小節が出る**。
//!
//! 小節境界では 1 バイトも送らない。境界で state load を出していたころは 100〜130ms の
//! 無音が毎小節でき、境界で note on を送っていたころは発音位置が −42.7〜+21.3ms
//! ぶれていた（資料の実測ログ）。

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
/// 分けてあるのは、**それぞれ意味が違う**ため。`preload_next` / `note_on_next` は
/// 小節の途中で起きるので音には出ない（が、小節長を超えたら先読みが破綻する）。
/// `prepare` / `note_on` は**小節境界で演奏スレッドが止まっていた時間**で、
/// 予約が当たっていれば両方 0 になる。混ぜて 1 つの数字にすると「先読みが効いて
/// いるのか、単に state load が軽かっただけなのか」が読めなくなる。
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct MeasureSendTiming {
    /// この小節の WAV が、1 つ前の小節の先読みで**既に載っていた**か。
    ///
    /// `false` になるのは演奏開始の 1 小節目と、先読みした小節と実際に進んだ小節が
    /// 食い違ったとき（演奏中の AB リピート変更・小節数変更）だけ。
    pub(crate) preloaded: bool,
    /// この小節を鳴らす位置（timeline 原点からのフレーム数）。
    ///
    /// **ジッタの判定材料はここ。** 隣り合う小節でこの差が小節長ちょうどなら、
    /// サーバーが実際に鳴らすサンプル位置の差も小節長ちょうどになる
    /// （サーバーの丸めは `round(秒 × サンプルレート)` で、フレームから作った秒は
    /// 必ず元のフレームへ戻る）。対策前はここが 100352〜103424 で揺れていた。
    pub(crate) at_frames: u64,
    /// 小節境界で state load に費やした実時間。
    ///
    /// **先読みが当たっていれば 0。** ここが 0 でない小節は、その値がそのまま
    /// 「小節の頭で演奏スレッドが止まっていた時間」になる。
    pub(crate) prepare: Duration,
    /// **小節境界で** note on の送信に費やした実時間。
    ///
    /// 予約が当たっていれば 0。境界で送るのは、先読みが外れて組み直したときだけ。
    pub(crate) note_on: Duration,
    /// この小節を鳴らしている最中に、**次の小節**をスロットへ載せるのに掛かった実時間。
    ///
    /// 先読みの重さはここに出る。小節長に対する占有率がそのまま読める。
    pub(crate) preload_next: Duration,
    /// **次の小節**の note on を timeline へ積むのに掛かった実時間。
    ///
    /// 全 track を 1 バッチで投げるので track 数に比例しない。ここが跳ねたら
    /// 「1 件ずつ送る形」へ戻った合図。発音位置は絶対秒で決まるので、この値が
    /// ぶれても音のタイミングは動かない。
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
/// `prepare_live_patch` は応答待ちでブロックする（1 件 10〜13ms・debug サーバーなら
/// 60〜85ms）ので、**鳴らす小節の 1 つ前の小節の中で**呼ぶこと。
///
/// `on_prepared` には「何本ぶん済んだか」を 1 本ごとに渡す。演奏開始の 1 小節目は
/// これがそのまま「音が鳴るまで」overlay の進捗になる（実測でここは
/// **1 本目だけ 1229ms、残り 6 本は 7〜11ms** と極端に偏るので、
/// 終わってからまとめて数えたのでは進んで見えない）。
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
        // patch 文字列が `.wav` で終わるので、サーバーは cache-player を選んで
        // instance を差し替える。`slot=N;` のプレフィクスが**どのスロットへ載せるか**を
        // 指定する（綴りの単一ソースは play server 側 `core-lib/src/cache_wav.rs`）。
        // state に入るのはパスであってファイルの中身ではない（1 ファイル約 1.6MB あるため）。
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

/// 載せ終えている全 track の note on を、**その小節の発音位置**へ timeline で予約する。
/// 戻り値は送信に掛かった実時間。
///
/// **ここで state load を出してはいけない。** 出すとその応答待ちのぶんだけ予約が遅れ、
/// しかも 1 track ずつ順に返ってくるので track ごとにバラバラの時刻で音が切り替わる
/// （対策前の実測: 8 track で 250ms、debug サーバーで 650ms）。
///
/// 発音位置は `at`（timeline 原点からのフレーム数）で、この関数が呼ばれた時刻ではない。
/// `at` は必ず [`MeasureTimeline`] から取ること。
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
