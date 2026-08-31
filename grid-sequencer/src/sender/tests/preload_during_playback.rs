//! **実物の [`GridMidiSender`] と実サーバーで、先読み中の送信キューを測る。**
//!
//! # なぜ realtime-play 側のテストでは足りないか
//! 実障害の一次証拠は `sender_queue_max_us=2928489` だった。これは
//! 「UI スレッドが送った `Send` が、送信スレッドに拾われるまで待たされた最大時間」で、
//! **grid の送信スレッドを通らないと発生しない値**。`realtime-play` の
//! `live_ipc/tests/timeline_during_preload.rs` は supervisor を直接叩くので、
//! この mpsc の詰まりは原理的に観測できない。ここはその1点のためだけにある。
//!
//! # 旧設計（v9）なら、なぜ落ちるか
//! v9 の送信スレッドは `GridMidiCommand::Preload` の分岐で patch load の完了を
//! 同期で待っていた。待っている間 `Send` は mpsc に溜まりっぱなしになり、
//! 拾われた時点の `queued_at.elapsed()` がロード時間そのもの（約 3 秒）になる。
//! 16 分音符の note off が 3 秒遅れれば、全音符に伸びて聞こえる。
//! [`QUEUE_BUDGET_US`] はその 1/10 未満に置いてあるので、同期待ちが戻れば必ず落ちる。
//!
//! # 耳の代わりになるもの
//! 「音が伸びて聞こえる」の機械的な言い換えは 2 つある。どちらもここで見る。
//!
//! - 送信キューの最大待ち時間（[`QUEUE_BUDGET_US`]）
//! - サーバーが数えた late event の増分（発音時刻を過ぎて届いたイベント数）
//!
//! 実測（Stage 5）: 送信スレッドを一時的に v9 の同期待ちへ戻すと
//! `sender_queue_max_us=1911919 late 0→58` で落ち、戻すと `1271` と `0→0` になった。
//!
//! ```text
//! $env:CMRT_TEST_PLAY_SERVER_EXE = "...\clap-mml-realtime-play-server.exe"
//! cargo test -p cmrt-grid-sequencer -- --include-ignored preload_during_playback --nocapture
//! ```

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use cmrt_realtime_play::{RealtimePlayServerSupervisor, TimelineMidiEvent};

use super::{
    super::GridMidiSender,
    test_play_server::{
        cfg_for_port, pick_port, TestPlayServer, PATCH_LOAD_DELAY_ENV, PLAY_SERVER_EXE_ENV,
    },
};

/// 2 track × 2 bank。grid の最小構成で bank 境界も先読みも成立する。
const INSTANCE_COUNT: usize = 4;
/// 待機 bank（bank 1）の先頭。
const STANDBY_INSTANCE: u8 = 2;
const TEMPO_BPM: f64 = 120.0;
/// 16 分音符 1 個ぶん（120BPM）。実障害で伸びたのがこの長さの音。
const STEP_SECONDS: f64 = 0.125;
/// 送信を発音時刻の何秒前に出すか。grid の実運用と同じく短く保つ。
const LOOKAHEAD: Duration = Duration::from_millis(200);
/// 人工ロードで止める長さ。実障害の 2.98 秒に近い窓を開ける。
const LOAD_DELAY_MS: u64 = 2_000;
/// 基準を取るまでに刻むステップ数（1 秒ぶん）。
const WARMUP_STEPS: usize = 8;
/// ロード中に最低これだけの送信が起きること。2 秒 ÷ 125ms = 16 なので半分を要求する。
const STEPS_DURING_LOAD_AT_LEAST: usize = 8;
/// 送信ループの安全弁。完了通知が来なくても無限に回らない。
const MAX_STEPS: usize = 240;
/// 送信キューの待ち時間の上限。**実障害は 2_928_489µs だった。**
/// 送信スレッドが塞がっていなければ、拾われるまでの待ちは
/// `recv_timeout` の 50ms（`METER_POLL_INTERVAL`）で頭打ちになる。
const QUEUE_BUDGET_US: u64 = 200_000;
/// サーバーが timing metrics を publish する周期（play-server の
/// `player/timing_diagnostics.rs` の `WINDOW`）。**late はこの周期でしか更新されない。**
/// これより短いテストだと late はずっと 0 のままで、何も判定していないことになる。
const METRICS_WINDOW: Duration = Duration::from_secs(5);
/// 1 回目の publish を跨ぐまで刻み続ける長さ。その window がロード区間を含む。
const PUMP_AT_LEAST: Duration = Duration::from_millis(6_500);

/// **2 秒の先読みロード中も、grid の送信キューが詰まらないこと。**
#[test]
#[ignore = "実機の play server 実行ファイルが要る（CMRT_TEST_PLAY_SERVER_EXE）"]
fn the_sender_queue_does_not_back_up_while_a_preload_is_still_loading() {
    let exe = std::env::var(PLAY_SERVER_EXE_ENV).unwrap_or_else(|_| {
        panic!("{PLAY_SERVER_EXE_ENV} に play server の実行ファイルを渡すこと")
    });
    // 起動中の TUI（既定 62154）とも、realtime-play 側のテスト（45_000〜53_999）とも
    // 衝突させない。
    let port = pick_port(54_000);
    let server = TestPlayServer::spawn(
        &exe,
        port,
        INSTANCE_COUNT,
        &[(PATCH_LOAD_DELAY_ENV, LOAD_DELAY_MS.to_string())],
    );

    let cfg = cfg_for_port(port);
    let supervisor = Arc::new(RealtimePlayServerSupervisor::with_live_instance_count(
        &cfg,
        INSTANCE_COUNT,
    ));
    supervisor
        .ensure_started_for_fast_midi()
        .unwrap_or_else(|error| {
            panic!(
                "起動済みサーバーへ繋がらない: {error:#} / {}",
                server.stderr_text()
            )
        });

    let sender = GridMidiSender::new(Arc::clone(&supervisor));
    // timeline id はプロセス内で通し番号なので、送るイベントにも同じ値を使う。
    let timeline_id = sender.begin_timeline(48_000.0, TEMPO_BPM);
    let outcome = run_load_window(&sender, &supervisor, timeline_id);
    // サーバーは先に落とす。判定で panic しても Drop が拾うが、
    // 落とした後の方が失敗時の出力が読みやすい。
    drop(sender);
    outcome.report();
    outcome.assert_queue_never_backed_up();
    drop(server);
}

/// 実測値。判定の前に必ず 1 行で出す。
struct SenderQueueOutcome {
    steps_during_load: usize,
    sender_queue_max_us: u64,
    preload_completed: usize,
    preload_total: usize,
    preload_failed: bool,
    late_before: u64,
    late_after: u64,
    load_elapsed: Duration,
    /// timeline を張ってから刻み終えるまでの実時間。[`METRICS_WINDOW`] 判定に使う。
    pumped: Duration,
}

/// warmup ののち先読みを 1 件重ね、完了まで 16 分音符を刻み続ける。
fn run_load_window(
    sender: &GridMidiSender,
    supervisor: &RealtimePlayServerSupervisor,
    timeline_id: u64,
) -> SenderQueueOutcome {
    let started = Instant::now();
    let mut late_before = 0;
    let mut load_started = None;
    let mut steps_during_load = 0;
    let mut load_elapsed = None;
    let mut sender_queue_max_us = 0;

    for step in 0..MAX_STEPS {
        let lateness = sleep_until_send_time(started, step);
        // UI スレッドの pump と同じ入口。ここは mpsc へ積むだけで即座に戻る。
        sender.send_scheduled(step_events(timeline_id, step), lateness);

        if step == WARMUP_STEPS {
            late_before = supervisor.timing_metrics().late_events_total;
            // 先読みサイクルを 1 件ぶん開ける。重みは人工ロードの長さ。
            sender.begin_preload_cycle(vec![LOAD_DELAY_MS]);
            sender.preload(STANDBY_INSTANCE, None);
            load_started = Some(Instant::now());
            continue;
        }

        let Some(load_began) = load_started else {
            continue;
        };
        let status = sender.status();
        // **毎ステップ拾うこと。** 送信スレッドは timing ログを出すたびに
        // この window をリセットするので、最後にまとめて読むと山を取り逃がす。
        sender_queue_max_us = sender_queue_max_us.max(status.sender_queue_max_us);
        if load_elapsed.is_none() {
            // ロード中。進捗が完了へ動くまで刻み続ける。
            steps_during_load += 1;
            if status.preload.completed >= status.preload.total.max(1) || status.preload_failed {
                load_elapsed = Some(load_began.elapsed());
                sender.finish_preload();
            }
            continue;
        }
        // ロードは終わったが、まだ刻む。サーバーの late は 5 秒ごとにしか publish
        // されないので、ロード区間を含む window が 1 回出るまで演奏を続けないと
        // 「遅れなかった」ことを確かめたつもりで 0 を読むだけになる。
        if started.elapsed() >= PUMP_AT_LEAST {
            break;
        }
    }

    let status = sender.status();
    SenderQueueOutcome {
        steps_during_load,
        sender_queue_max_us,
        preload_completed: status.preload.completed,
        preload_total: status.preload.total,
        preload_failed: status.preload_failed,
        late_before,
        late_after: supervisor.timing_metrics().late_events_total,
        load_elapsed: load_elapsed.unwrap_or_default(),
        pumped: started.elapsed(),
    }
}

impl SenderQueueOutcome {
    fn report(&self) {
        eprintln!(
            "preload-during-playback: steps_during_load={} sender_queue_max_us={} \
             preload={}/{} failed={} late {}→{} load_elapsed={:?} pumped={:?}",
            self.steps_during_load,
            self.sender_queue_max_us,
            self.preload_completed,
            self.preload_total,
            self.preload_failed,
            self.late_before,
            self.late_after,
            self.load_elapsed,
            self.pumped,
        );
    }

    fn assert_queue_never_backed_up(&self) {
        // 人工遅延が効いていること。効いていなければ以下は何も見ていない。
        assert!(
            self.load_elapsed >= Duration::from_millis(LOAD_DELAY_MS),
            "人工遅延が効いていない（先読みが {:?} で終わった）",
            self.load_elapsed
        );
        assert!(!self.preload_failed, "先読みが失敗した");
        assert_eq!(
            self.preload_completed, self.preload_total,
            "先読みの進捗が完了になっていない"
        );
        assert!(
            self.steps_during_load >= STEPS_DURING_LOAD_AT_LEAST,
            "ロード中に {} ステップしか送れていない",
            self.steps_during_load
        );
        // 本題。実障害ではここが 2_928_489 だった。
        assert!(
            self.sender_queue_max_us < QUEUE_BUDGET_US,
            "送信キューが {}µs 詰まった（送信スレッドがロードを待っている）",
            self.sender_queue_max_us
        );
        // late を読むだけの value があること。publish を跨いでいなければ、
        // 次の assert は 0 と 0 を比べるだけで何も見ていない。
        assert!(
            self.pumped >= METRICS_WINDOW,
            "timing metrics の publish 周期（{METRICS_WINDOW:?}）を跨いでいない: {:?}",
            self.pumped
        );
        // 詰まれば note off が発音時刻を過ぎて届く。実障害では late が 260 増えた。
        assert_eq!(
            self.late_after, self.late_before,
            "先読み中にイベントが遅れた"
        );
    }
}

/// `step` ぶんの送信時刻まで眠り、そこからの遅れ（pump lateness）を返す。
fn sleep_until_send_time(started: Instant, step: usize) -> Duration {
    let due = Duration::from_secs_f64(step as f64 * STEP_SECONDS).saturating_sub(LOOKAHEAD);
    let elapsed = started.elapsed();
    if due > elapsed {
        std::thread::sleep(due - elapsed);
        Duration::ZERO
    } else {
        elapsed - due
    }
}

/// 16 分音符 1 ステップぶんの note on / note off。bank 0 の 2 instance が鳴る。
fn step_events(timeline_id: u64, step: usize) -> Vec<TimelineMidiEvent> {
    let at = step as f64 * STEP_SECONDS;
    let mut events = Vec::new();
    for instance_id in 0..2u8 {
        let key = 60 + (step % 12) as u8 + instance_id * 12;
        events.push(TimelineMidiEvent {
            timeline_id,
            instance_id,
            timeline_seconds: at,
            message: [0x90, key, 100],
        });
        events.push(TimelineMidiEvent {
            timeline_id,
            instance_id,
            // 次のステップへ食い込ませない。
            timeline_seconds: at + STEP_SECONDS * 0.9,
            message: [0x80, key, 0],
        });
    }
    events
}
