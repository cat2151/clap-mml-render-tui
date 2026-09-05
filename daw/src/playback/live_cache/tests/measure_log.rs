use std::time::Duration;

use super::wav;
use crate::playback::live_cache::{
    format_live_cache_measure_log, measure_live_cues, MeasureSendTiming,
};

/// ログ 1 行だけで「送った / 無音 / 上限超え」の内訳と、先読みが効いたかが読める。
///
/// `preload=hit` なら小節境界では 1 バイトも送っていない（`prepare_ms` も
/// `note_on_ms` も 0.0）。state load も note on の予約も
/// `next_ms` / `next_note_on_ms` の側（1 つ前の小節の途中）へ移っている。
#[test]
fn a_preloaded_measure_spends_no_time_at_the_boundary() {
    let cues = measure_live_cues(5, |row| (row != 3).then(|| wav(row)));

    assert_eq!(
        format_live_cache_measure_log(
            2,
            3,
            &cues,
            MeasureSendTiming {
                preloaded: true,
                at_frames: 240_000,
                prepare: Duration::ZERO,
                note_on: Duration::ZERO,
                preload_next: Duration::from_micros(105_300),
                note_on_next: Duration::from_micros(120),
            }
        ),
        "meas3: live-cache slot=2 preload=hit sent=row2/i0,row4/i2 silent=row3 over_limit=- \
         at_frames=240000 prepare_ms=0.0 note_on_ms=0.0 next=meas4/slot3 next_ms=105.3 \
         next_note_on_ms=0.1"
    );
}

/// 先読みが外れた小節は、境界で止まった時間がそのまま出る。
///
/// これが出るのは演奏開始の 1 小節目と、演奏中に AB リピート・小節数が変わったときだけ。
/// 毎小節 `miss` が並んでいたら先読みが機能していない。
#[test]
fn a_missed_preload_shows_the_time_it_blocked_at_the_boundary() {
    let cues = measure_live_cues(3, |row| Some(wav(row)));

    assert_eq!(
        format_live_cache_measure_log(
            0,
            1,
            &cues,
            MeasureSendTiming {
                preloaded: false,
                at_frames: 12_000,
                prepare: Duration::from_micros(108_600),
                note_on: Duration::from_micros(700),
                preload_next: Duration::from_micros(101_200),
                note_on_next: Duration::from_micros(200),
            }
        ),
        "meas1: live-cache slot=0 preload=miss sent=row2/i0 silent=- over_limit=- \
         at_frames=12000 prepare_ms=108.6 note_on_ms=0.7 next=meas2/slot1 next_ms=101.2 \
         next_note_on_ms=0.2"
    );
}

/// 何も送っていない小節でも「空だった」ことが 1 行で分かる。
#[test]
fn the_measure_log_line_uses_a_dash_when_a_category_is_empty() {
    let cues = measure_live_cues(4, |_| None);

    assert_eq!(
        format_live_cache_measure_log(0, 1, &cues, MeasureSendTiming::default()),
        "meas1: live-cache slot=0 preload=miss sent=- silent=row2,row3 over_limit=- \
         at_frames=0 prepare_ms=0.0 note_on_ms=0.0 next=meas2/slot1 next_ms=0.0 \
         next_note_on_ms=0.0"
    );
}

/// ループの端では「次の小節」が先頭へ折り返す。ログでもそれが読める。
///
/// 自前で `+1` すると `meas3` の次が `meas4`（存在しない小節）になり、
/// 先読みが毎周 1 回ずつ無駄になったうえで `preload=miss` が並ぶ。
///
/// **ここは折り返しでスロットが衝突しない例**（3 小節ループ / 4 スロット）。
/// スロットが 2 本だった頃は `meas3` も `meas1` もスロット 0 で、
/// 折り返しの先読みが鳴っている小節を踏んでいた。衝突する組み合わせと
/// その余裕は [`super::slot_headroom`] が数で押さえている。
#[test]
fn the_last_measure_preloads_the_first_one_of_the_loop() {
    let cues = measure_live_cues(3, |row| Some(wav(row)));
    let line = format_live_cache_measure_log(2, 0, &cues, MeasureSendTiming::default());

    assert!(line.starts_with("meas3: live-cache slot=2 "), "{line}");
    assert!(line.contains(" next=meas1/slot0 "), "{line}");
}
