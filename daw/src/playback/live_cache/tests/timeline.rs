//! 発音位置のグリッド。実サーバーもファイルも要らない。
//!
//! ここで固定するのは 1 つだけ。**予約した位置が、サーバーの丸めを通しても
//! 小節長ちょうどの間隔で並ぶこと。** 対策前は「届いた瞬間のオーディオブロック」で
//! 鳴らしていたので、実測で 100352〜103424 サンプル（理想 102400）ぶれていた。

use std::time::{Duration, Instant};

use crate::playback::live_cache::timeline::MeasureTimeline;

const SAMPLE_RATE: u32 = 48_000;

/// play server が実際に使う丸め（`SampleRate::seconds_to_sample` = `round()`）。
///
/// **予約した秒をここへ通した値が「実際に鳴るサンプル位置」**なので、判定は
/// フレーム数そのものではなくこの値でやる。フレームで積んでも、秒へ直す割り算で
/// 誤差が出れば戻ってこない。
fn sounding_sample(timeline: &MeasureTimeline, at: u64) -> u64 {
    (timeline.seconds_of(at) * f64::from(SAMPLE_RATE)).round() as u64
}

/// 連続して予約した小節の**発音サンプル位置の差**を並べる。
fn intervals(timeline: &MeasureTimeline, positions: &[u64]) -> Vec<u64> {
    positions
        .windows(2)
        .map(|pair| sounding_sample(timeline, pair[1]) - sounding_sample(timeline, pair[0]))
        .collect()
}

/// 演奏ループと同じ順番で `measures` 小節ぶん予約する。
///
/// **実際のループは「小節 N の頭で小節 N+1 を予約する」**（`live_cache.rs` の `run()`）。
/// `now` をそれ以外の刻みで進めると、この関数が試しているのとは別のことを試すことになる。
/// 待ちのオーバーシュートぶん（0〜6ms）を混ぜてあるのは、**そこがぶれても発音位置は
/// 動かない**ことを見るため。
fn walk(timeline: &mut MeasureTimeline, measures: u64, measure_frames: u64) -> Vec<u64> {
    let mut positions = vec![timeline.restart_at(Instant::now(), measure_frames)];
    for measure in 1..measures {
        let previous = *positions.last().expect("1 小節目は入っている");
        let now = timeline.instant_of(previous) + Duration::from_millis(measure % 7);
        positions.push(timeline.reserve(now, measure_frames));
    }
    positions
}

/// 小節が小節長ちょうどの間隔で並ぶ。**この Stage の本題。**
///
/// `now` を毎回進めても位置は動かない。位置を決めているのはグリッドであって
/// 「いつ予約したか」ではない、というのが timeline 化の中身。
#[test]
fn every_measure_lands_exactly_one_measure_after_the_previous_one() {
    // BPM 130 の 4 拍 = 102400 フレーム（資料の実測ログと同じ小節長）。
    const MEASURE_FRAMES: u64 = 102_400;
    let mut timeline = MeasureTimeline::for_test(Instant::now(), SAMPLE_RATE);

    let positions = walk(&mut timeline, 64, MEASURE_FRAMES);

    let intervals = intervals(&timeline, &positions);
    assert!(
        intervals.iter().all(|interval| *interval == MEASURE_FRAMES),
        "小節の間隔が揺れている: {intervals:?}"
    );
}

/// **小節長がナノ秒で割り切れなくても**間隔は 1 サンプルも揺れない。
///
/// ここが `Duration`（整数ナノ秒）や `f64` 秒で積む実装だと落ちる。例えば
/// 4 拍 BPM137 は 84088 フレーム = 1.751833…秒で、ナノ秒へ丸めた値を積むと
/// サーバーの `round()` が 84087 と 84088 を交互に返す。
#[test]
fn a_measure_length_that_is_not_a_whole_number_of_nanoseconds_still_never_drifts() {
    // 4 拍 BPM137 @48kHz。1 フレーム = 20833.333…ns。
    const MEASURE_FRAMES: u64 = 84_088;
    let mut timeline = MeasureTimeline::for_test(Instant::now(), SAMPLE_RATE);

    let positions = walk(&mut timeline, 512, MEASURE_FRAMES);

    let intervals = intervals(&timeline, &positions);
    assert!(
        intervals.iter().all(|interval| *interval == MEASURE_FRAMES),
        "小節の間隔が揺れている: {:?}",
        intervals
            .iter()
            .filter(|interval| **interval != MEASURE_FRAMES)
            .collect::<Vec<_>>()
    );
}

/// 予約は必ず「いま」より先。ここが 0 だと、届いた時点で過ぎている＝サーバーが
/// ブロックの頭へクランプし、対策前と同じジッタに戻る。
#[test]
fn the_first_measure_is_reserved_ahead_of_now_instead_of_at_zero() {
    const MEASURE_FRAMES: u64 = 102_400;
    let origin = Instant::now();
    let mut timeline = MeasureTimeline::for_test(origin, SAMPLE_RATE);

    let at = timeline.restart_at(origin, MEASURE_FRAMES);

    assert!(at > 0, "「いますぐ」で予約している");
    // 先行時間の上限は 250ms（小節長の半分が下回ればそちら）。
    assert_eq!(at, 250 * u64::from(SAMPLE_RATE) / 1_000);
}

/// 小節が短いときは、先行時間を小節長の半分まで縮める。
///
/// 縮めないと「いまから 250ms 後」が次の小節の予約位置より後ろになり、
/// 毎小節グリッドを張り直して（＝ジッタが戻って）しまう。
#[test]
fn a_short_measure_shrinks_the_lead_so_the_grid_survives() {
    // 0.25 秒の小節。先行時間は半分の 0.125 秒。
    const MEASURE_FRAMES: u64 = 12_000;
    let mut timeline = MeasureTimeline::for_test(Instant::now(), SAMPLE_RATE);

    let positions = walk(&mut timeline, 16, MEASURE_FRAMES);

    assert_eq!(positions[0], MEASURE_FRAMES / 2, "先行時間が縮んでいない");
    let intervals = intervals(&timeline, &positions);
    assert!(
        intervals.iter().all(|interval| *interval == MEASURE_FRAMES),
        "短い小節でグリッドが張り直されている: {intervals:?}"
    );
}

/// 先読みが外れた小節は「いまから」へ張り直す。
///
/// [`MeasureTimeline::reserve`] をそのまま使うと、外れた小節ぶん先の位置が返って
/// **1 小節まるごと無音**になる。
#[test]
fn a_missed_preload_re_anchors_the_grid_instead_of_waiting_a_whole_measure() {
    const MEASURE_FRAMES: u64 = 102_400;
    let origin = Instant::now();
    let mut timeline = MeasureTimeline::for_test(origin, SAMPLE_RATE);

    // 小節 1 と小節 2 を予約したところで、小節 2 の境界に着いた（≒ 2.133 秒後）。
    timeline.restart_at(origin, MEASURE_FRAMES);
    let planned = timeline.reserve(origin, MEASURE_FRAMES);
    let boundary = origin + Duration::from_millis(2_383);

    let restarted = timeline.restart_at(boundary, MEASURE_FRAMES);

    assert!(
        restarted < planned + MEASURE_FRAMES,
        "1 小節先へ飛んでいる（その間ずっと無音になる）"
    );
    // 境界のいまから先行時間ぶんだけ先。
    assert_eq!(
        restarted,
        (2.383 * f64::from(SAMPLE_RATE)) as u64 + 250 * u64::from(SAMPLE_RATE) / 1_000
    );
}

/// 演奏スレッドが 1 小節ぶん近く遅れたら、予約は「いまから」へ逃がす。
///
/// 逃がさないと過去の位置を予約し続け、サーバーが全部ブロック頭へクランプする。
#[test]
fn a_late_play_thread_falls_back_to_re_anchoring() {
    const MEASURE_FRAMES: u64 = 12_000; // 0.25 秒
    let origin = Instant::now();
    let mut timeline = MeasureTimeline::for_test(origin, SAMPLE_RATE);

    timeline.restart_at(origin, MEASURE_FRAMES);
    // 次の予約位置は 0.375 秒だが、実時間は既に 1 秒進んでいる。
    let at = timeline.reserve(origin + Duration::from_secs(1), MEASURE_FRAMES);

    assert!(
        at >= u64::from(SAMPLE_RATE),
        "過去の位置を予約している: {at}"
    );
}

/// 予約位置と実時刻の対応が保たれる（演奏位置の表示と小節境界の待ちが使う）。
#[test]
fn the_wall_clock_instant_of_a_position_matches_the_position() {
    const MEASURE_FRAMES: u64 = 102_400;
    let origin = Instant::now();
    let timeline = MeasureTimeline::for_test(origin, SAMPLE_RATE);

    assert_eq!(timeline.instant_of(0), origin);
    assert_eq!(
        timeline.instant_of(MEASURE_FRAMES),
        origin + Duration::from_secs_f64(f64::from(102_400) / f64::from(SAMPLE_RATE))
    );
}
