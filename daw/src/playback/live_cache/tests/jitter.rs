//! 実サーバーで**発音位置のジッタが 0** であることを見る、通常は skip されるテスト。
//!
//! この Stage の受け入れ条件そのもの。対策前は小節の間隔が 100352〜103424 サンプル
//! （理想 102400、**振れ幅 64ms = 3 オーディオブロック**）でぶれていた。
//!
//! ```text
//! CMRT_REALTIME_PLAY_SERVER_PORT=8712 CMRT_LIVE_INSTANCE_COUNT=2 \
//!   ../clap-mml-play-server/target/debug/clap-mml-realtime-play-server.exe > server.log 2>&1 &
//! CMRT_LIVE_CACHE_TEST_PORT=8712 CMRT_LIVE_CACHE_TEST_WAV=<絶対パス> \
//!   cargo test -p cmrt-daw --lib jitter -- --test-threads=1 --nocapture
//! ```
//!
//! ## なぜサーバーログを読まないのか
//!
//! 資料の実測は `cmrt-live: event=apply-midi ... clock=N` を数えていたが、**この行は
//! 生 live MIDI の経路にしか無い**（timeline 経路には出ない）。代わりに次の 2 つで
//! 同じことを言い切れる。
//!
//! 1. こちらが予約した位置（ログの `at_frames`）が小節長ちょうどの間隔で並ぶ
//! 2. サーバーが 1 件も late にしていない（`timing_metrics().late_events_total == 0`）
//!
//! 1 だけだと「予約はきれいだが届くのが遅れてブロック頭へクランプされた」を見逃す。
//! 2 だけだと「late ではないが予約自体がぶれている」を見逃す。**2 つ揃って初めて
//! 「実際に鳴ったサンプル位置の差が小節長ちょうど」になる。** サーバーは late で
//! ない予約を `round(秒 × サンプルレート)` の位置へそのまま置くため
//! （play server 側 `BlockScheduler::take_block`）。
//!
//! `timing_metrics()` は 5 秒周期でしか共有メモリへ載らない（play server 側
//! `LiveTimingWindow::WINDOW`）ので、**窓が 1 回は閉じる長さ**演奏すること。

use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::playback::{live_gain::live_track_gains, real_server::real_server_from_env};

const SAMPLE_RATE: u32 = 48_000;
/// 小節長。短くしすぎると先読みが小節に収まらず、長くすると窓が閉じるまで待たされる。
const MEASURE_SECONDS: f64 = 1.0;
/// ループの長さ。窓（5 秒）より長く演奏するので、これを何周かする。
const MEASURES: usize = 4;
/// 演奏する長さ。`LiveTimingWindow` の窓（5 秒）が最低 1 回閉じること。
const PLAY_SECONDS: f64 = 7.0;

/// 小節ログから `at_frames=` を拾う（`u64` なので誤差の入る余地が無い）。
fn measure_positions(log_lines: &[String]) -> Vec<u64> {
    log_lines
        .iter()
        .filter(|line| line.contains(": live-cache "))
        .filter_map(|line| line.split(" at_frames=").nth(1))
        .map(|rest| {
            rest.split_whitespace()
                .next()
                .expect("値が続くこと")
                .parse()
                .expect("at_frames は整数")
        })
        .collect()
}

/// 小節の発音位置が、小節長ちょうどの間隔で並ぶ。
///
/// **壊して赤くなるか**: `MeasureTimeline::reserve` を「毎回 `restart_at` する」形へ
/// 変えると、間隔が実時間のぶれをそのまま拾って揺れる（＝対策前の状態）。
#[test]
fn a_real_server_sounds_every_measure_exactly_one_measure_apart() {
    let Some((play_server, wav)) = real_server_from_env() else {
        // 実サーバーが無い環境では何もしない（CI でも常に green）。
        return;
    };

    let measure_samples = (MEASURE_SECONDS * f64::from(SAMPLE_RATE) * 2.0) as usize;
    let measure_frames = (measure_samples / 2) as u64;
    let play_state = Arc::new(Mutex::new(crate::DawPlayState::Playing));
    let log_lines = Arc::new(Mutex::new(VecDeque::new()));
    let play_server = Arc::new(play_server);

    // 先に 1 コマンド送って SHM へ繋いでおく。未接続の `timing_metrics()` は
    // 既定値を返すので、繋ぐ前に読むと判定材料にならない。
    play_server
        .set_live_auto_gain_enabled(false)
        .expect("SHM へ繋げること");

    let play_loop = crate::playback::live_cache::LiveCachePlayLoop {
        play_server: Arc::clone(&play_server),
        play_state: Arc::clone(&play_state),
        play_position: Arc::new(Mutex::new(None)),
        ab_repeat: Arc::new(Mutex::new(crate::AbRepeatState::Off)),
        measure_mmls: Arc::new(Mutex::new(vec!["cde".to_string(); MEASURES])),
        measure_samples: Arc::new(Mutex::new(measure_samples)),
        log_lines: Arc::clone(&log_lines),
        sample_rate: SAMPLE_RATE,
        tempo_bpm: 120.0,
        beat_numerator: 4,
        tracks: crate::tracks::FIRST_PLAYABLE_TRACK + 1,
        ready_cache_wav: Arc::new(move |_measure_index, _row| Some(wav.clone())),
        initial_track_gains: live_track_gains(
            crate::tracks::FIRST_PLAYABLE_TRACK + 1,
            |_| -3,
            |_| true,
        ),
        sent_track_gains: Arc::new(Mutex::new(Vec::new())),
        startup: crate::playback::DawPlaybackStartupState::default(),
    };

    let handle = std::thread::spawn(move || play_loop.run(0));
    std::thread::sleep(Duration::from_secs_f64(PLAY_SECONDS));
    let timing = play_server.timing_metrics();
    *play_state.lock().unwrap() = crate::DawPlayState::Idle;
    handle.join().expect("演奏ループが停止すること");

    let logged: Vec<String> = log_lines.lock().unwrap().iter().cloned().collect();
    let failures: Vec<&String> = logged
        .iter()
        .filter(|line| line.contains("failed"))
        .collect();
    assert!(failures.is_empty(), "送信に失敗した行がある: {failures:?}");
    assert!(
        logged
            .iter()
            .any(|line| line.contains("timeline begin ") && line.ends_with("result=ok")),
        "timeline を張れていない（これが無いと note on は全部捨てられる）: {logged:?}"
    );

    let positions = measure_positions(&logged);
    let intervals: Vec<i64> = positions
        .windows(2)
        .map(|pair| pair[1] as i64 - pair[0] as i64)
        .collect();
    eprintln!(
        "live-cache jitter: measure_frames={measure_frames} positions={positions:?} \
         intervals={intervals:?} timing_events={} late={} late_total={} \
         max_late_samples={} max_late_us={:.1}",
        timing.events,
        timing.late_events,
        timing.late_events_total,
        timing.max_late_samples,
        timing.max_late_us,
    );

    assert!(
        intervals.len() >= 4,
        "小節が進んでいない（{} 小節ぶんしか予約していない）",
        positions.len()
    );
    // ── この Stage の本題 ──────────────────────────────────────
    assert!(
        intervals
            .iter()
            .all(|interval| *interval == measure_frames as i64),
        "小節の発音位置が小節長ちょうどで並んでいない: {intervals:?} \
         （小節長 {measure_frames} フレーム）"
    );
    // サーバーが 1 件も取りこぼしていない＝予約どおりのサンプル位置で鳴っている。
    // ここが 0 でないと、その件数ぶんは「ブロックの頭」へ寄せられている
    // （＝対策前と同じ、最大 1 ブロックのジッタが残っている）。
    assert_eq!(
        timing.late_events_total, 0,
        "予約に間に合っていない note on がある（最大 {} サンプル遅れ）",
        timing.max_late_samples
    );
    // timeline 経路を通ったことの裏取り。`observe_events` は timeline のイベントしか
    // 数えないので、生 live MIDI へ戻ると 0 になる。
    assert!(
        timing.events > 0,
        "サーバーが timeline のイベントを 1 件も見ていない（生 live MIDI へ戻っている）"
    );

    play_server
        .stop_live_all()
        .expect("停止できること（次のテストへ音を残さない）");
}
