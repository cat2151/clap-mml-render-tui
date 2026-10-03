//! 長い SMF 1 曲ぶんの timeline を一気に積んでも、コマンド枠もサーバーの待ち行列も
//! あふれないこと。あふれたときは、捨てた数がこちらへ届くこと。
//!
//! 件数は `solo_violin.mid`（379 秒）と同じ 16469 件。128 件ずつ、待たずに送る。
//!
//! ```text
//! $env:CMRT_TEST_PLAY_SERVER_EXE = "...\clap-mml-realtime-play-server.exe"
//! cargo test -p cmrt-realtime-play -- --include-ignored --test-threads=1 long_timeline --nocapture
//! ```

use std::time::{Duration, Instant};

use super::harness::{pick_port, TestPlayServer, PLAY_SERVER_EXE_ENV};
use crate::fast_midi_ipc::FastIpcError;
use crate::{LiveTimelineConfig, TimelineMidiEvent, MAX_MIDI_MESSAGES};

const INSTANCE_COUNT: usize = 2;
const TIMELINE_ID: u64 = 7;
const SMF_EVENTS: usize = 16_469;
/// サーバーの待ち行列の上限（`MAX_LIVE_QUEUE_EVENTS`）。
const SERVER_QUEUE_LIMIT: usize = 65_536;
/// 積んだイベントが待ち行列に残っている間に、サーバーが捨てた数を読むまでの待ち。
const SETTLE: Duration = Duration::from_millis(500);
/// コマンド枠が満杯のとき、送り直すまでの待ち。
const QUEUE_FULL_RETRY: Duration = Duration::from_millis(2);

#[test]
#[ignore = "実機の play server 実行ファイルが要る（CMRT_TEST_PLAY_SERVER_EXE）"]
fn a_whole_smf_fits_the_server_queue_and_overflow_is_reported() {
    let exe = std::env::var(PLAY_SERVER_EXE_ENV).unwrap_or_else(|_| {
        panic!("{PLAY_SERVER_EXE_ENV} に play server の実行ファイルを渡すこと")
    });
    // 起動中の TUI の既定ポートとも、他のテストのサーバーとも衝突させない。
    let port = pick_port(55_000);
    let server = TestPlayServer::spawn(&exe, port, INSTANCE_COUNT);
    let cfg = crate::tests::cfg_for_port(port);
    let supervisor =
        crate::RealtimePlayServerSupervisor::with_live_instance_count(&cfg, INSTANCE_COUNT);
    supervisor
        .ensure_started_for_fast_midi()
        .expect("起動済みサーバーへ繋がらない");
    supervisor
        .begin_live_timeline(LiveTimelineConfig {
            timeline_id: TIMELINE_ID,
            sample_rate_hz: 48_000.0,
            tempo_bpm: 120.0,
            time_signature_numerator: 4,
            time_signature_denominator: 4,
        })
        .expect("timeline を張れない");
    let dropped_before = supervisor.dropped_live_events_total();

    let started = Instant::now();
    let smf = send_all(&supervisor, &events(0, SMF_EVENTS));
    let elapsed = started.elapsed();
    std::thread::sleep(SETTLE);
    let dropped_after_smf = supervisor.dropped_live_events_total();
    println!(
        "long-timeline: events={SMF_EVENTS} batches={} queue_full={} elapsed_ms={} dropped {}→{}",
        smf.batches,
        smf.queue_full,
        elapsed.as_millis(),
        dropped_before,
        dropped_after_smf
    );
    assert_eq!(smf.queue_full, 0, "コマンド枠が満杯になった");
    assert_eq!(
        dropped_after_smf,
        dropped_before,
        "1 曲ぶんでサーバーが捨てた: {}",
        server.stderr_text()
    );

    // 同じ timeline の先へ、上限を超えるまで積み足す。捨てた数がこちらへ届くことを見る。
    let overflow = SERVER_QUEUE_LIMIT + 1_000 - SMF_EVENTS;
    let extra = send_all(&supervisor, &events(SMF_EVENTS, overflow));
    std::thread::sleep(SETTLE);
    let dropped_after_overflow = supervisor.dropped_live_events_total() - dropped_after_smf;
    println!(
        "long-timeline: overflow events={overflow} batches={} queue_full={} dropped={dropped_after_overflow}",
        extra.batches, extra.queue_full
    );
    // 積んでいる間にも先頭から鳴って待ち行列を出ていくので、捨てる数は 1000 以下になる。
    assert!(
        (1..=1_000).contains(&dropped_after_overflow),
        "上限を 1000 件超えて積んだのに、捨てた数が {dropped_after_overflow}: {}",
        server.stderr_text()
    );
}

struct Sent {
    batches: usize,
    queue_full: usize,
}

/// 128 件ずつ待たずに送る。満杯なら少し待って同じバッチを送り直し、その回数を数える。
fn send_all(
    supervisor: &crate::RealtimePlayServerSupervisor,
    events: &[TimelineMidiEvent],
) -> Sent {
    let mut sent = Sent {
        batches: 0,
        queue_full: 0,
    };
    for batch in events.chunks(MAX_MIDI_MESSAGES) {
        loop {
            match supervisor.send_timeline_events(batch) {
                Ok(_) => break,
                Err(error) if error.downcast_ref() == Some(&FastIpcError::QueueFull) => {
                    sent.queue_full += 1;
                    std::thread::sleep(QUEUE_FULL_RETRY);
                }
                Err(error) => panic!("timeline イベントを送れない: {error:#}"),
            }
        }
        sent.batches += 1;
    }
    sent
}

/// `first` 番目から `count` 件。note on と note off が交互に並ぶ、23 ms 間隔の列。
fn events(first: usize, count: usize) -> Vec<TimelineMidiEvent> {
    (first..first + count)
        .map(|index| {
            let pitch = 40 + (index / 2 % 40) as u8;
            let message = if index % 2 == 0 {
                [0x90, pitch, 100]
            } else {
                [0x80, pitch, 0]
            };
            TimelineMidiEvent {
                timeline_id: TIMELINE_ID,
                instance_id: 0,
                timeline_seconds: 1.0 + index as f64 * 0.023,
                message,
            }
        })
        .collect()
}
