//! **先読みのロード中も timeline イベントが届き続けることを、実サーバーで固定する。**
//!
//! # 何の番人か
//! 2026-08-31 の実障害そのもの。待機 bank へ重い音色を先読みすると、演奏 bank の
//! render は続いているのに live timeline の MIDI 供給だけが止まり、先読み済み
//! イベントを使い切った後の note off が遅れて「16 分音符が全音符まで伸びる」。
//! 実ログでは `sender_queue_max_us=2928489`、直後の metrics が `late=260` だった。
//!
//! # 旧設計（protocol v9）なら、なぜ落ちるか
//! v9 では 1 件の先読みが 2 か所を同時に塞いでいた。
//!
//! 1. **クライアント側**: `prepare_standby_patch` がロード完了まで戻らない。
//!    このテストの送信ループは先読みを出した後もステップを刻み続けるので、
//!    塞がれば [`STEPS_DURING_LOAD_AT_LEAST`] 件の送信が 1 件も起きない。
//! 2. **サーバー側**: fast IPC の受信ループが `PrepareStandbyPatch` の dispatch の
//!    中で完了を待つ。塞がれば `event=start` と `event=finish` の**間**に
//!    `cmrt-ipc-recv: kind=timeline-midi` が 1 行も現れない。
//!
//! 2 は「行が出たか」では判定できない。ロード完了後にまとめて届いても同じ行が出る
//! からで、区別できるのは順序だけ（[`count_lines_between`]）。
//!
//! 実測（Stage 5）: サーバーの受付を一時的に v9 の同期待ちへ戻すと
//! `accept=2.0183747s steps_during_load=1 timeline_midi_between=0` で落ちた。
//! 直した後は `accept=1.0932ms steps_during_load=17 timeline_midi_between=16`。
//!
//! `late_events_total` も見るが、こちらが跳ねるのを実測したのは grid 側
//! （`cmrt-grid-sequencer` の `sender::tests::preload_during_playback`、同期待ちで
//! `late 0→58`）。**late はサーバーが 5 秒周期でしか publish しない**ので、
//! [`PUMP_AT_LEAST`] まで刻み続けないと 0 を読むだけで何も見ないことになる。
//!
//! # 既存テストとの違い
//! `grid_cycle.rs` は 1 周ぶんのイベントを**先読みの前に全部**送ってしまうので、
//! ロード中に供給が止まっても metrics が悪化しない。あちらは underrun の回帰用で、
//! 今回の supply starvation の番人にはならない。ここは実運用と同じく、短い
//! lookahead で刻み続けながら先読みを重ねる。
//!
//! ```text
//! $env:CMRT_TEST_PLAY_SERVER_EXE = "...\clap-mml-realtime-play-server.exe"
//! cargo test -p cmrt-realtime-play -- --include-ignored timeline_during_preload --nocapture
//! ```

use std::time::{Duration, Instant};

use crate::{LiveTimelineConfig, TimelineMidiEvent};

use super::harness::{
    count_lines_between, number_field, pick_port, TestPlayServer, PATCH_LOAD_DELAY_ENV,
    PLAY_SERVER_EXE_ENV,
};

/// 2 track × 2 bank。grid の最小構成で、bank 境界も先読みも成立する。
const INSTANCE_COUNT: usize = 4;
/// 待機 bank（bank 1）の先頭。ここへ先読みする。
const STANDBY_INSTANCE: u8 = 2;
/// 待機 bank の 2 件目。完了後に次を始められることの確認用。
const NEXT_STANDBY_INSTANCE: u8 = 3;
const TIMELINE_ID: u64 = 1;
const TEMPO_BPM: f64 = 120.0;
/// 16 分音符 1 個ぶん（120BPM）。実障害で伸びたのがこの長さの音。
const STEP_SECONDS: f64 = 0.125;
/// 送信を発音時刻より何秒前に出すか。**grid と同じく短い**。
///
/// ここを厚くすると、供給が数百 ms 止まっても遅れが表に出なくなる。不変条件 3
/// 「late を大きな lookahead で隠さない」はこの値のこと。
const LOOKAHEAD: Duration = Duration::from_millis(200);
/// 人工ロードで止める長さ。実障害の 2.98 秒に近い窓を開ける。
const LOAD_DELAY_MS: u64 = 2_000;
/// 基準 metrics を取るまでに刻むステップ数（1 秒ぶん）。
const WARMUP_STEPS: usize = 8;
/// ロード中に最低これだけの送信が起きること。2 秒 ÷ 125ms = 16 なので半分を要求する。
const STEPS_DURING_LOAD_AT_LEAST: usize = 8;
/// 送信ループの安全弁。完了通知が来なくても無限に回らない。
const MAX_STEPS: usize = 240;
/// grid が実運用で使う出力バッファの厚さ（`grid-sequencer` の `INITIAL_BUFFER_MULTIPLIER`）。
/// 512 フレーム ÷ 48kHz × 2 ≒ 21ms しか余裕がない。
const BUFFER_MULTIPLIER: u16 = 2;
/// 受付応答と 1 回の送信に許す時間。IPC の 1 往復ぶんしかかからないはず。
const CALL_BUDGET: Duration = Duration::from_millis(300);
/// サーバーが timing metrics を publish する周期（play-server の
/// `player/timing_diagnostics.rs` の `WINDOW`）。**late はこの周期でしか更新されない。**
/// これより短いテストだと late は最後まで 0 のままで、何も判定していないことになる。
const METRICS_WINDOW: Duration = Duration::from_secs(5);
/// 1 回目の publish を跨ぐまで刻み続ける長さ。その window がロード区間を含む。
const PUMP_AT_LEAST: Duration = Duration::from_millis(6_500);

/// 実測値。`--nocapture` で読めるよう、判定の前に必ず 1 行で出す。
struct LoadWindow {
    accept: Duration,
    steps_during_load: usize,
    max_send_during_load: Duration,
    underrun_before: u64,
    underrun_after: u64,
    late_before: u64,
    late_after: u64,
    timeline_midi_between: usize,
    /// timeline を張ってから刻み終えるまでの実時間。[`METRICS_WINDOW`] 判定に使う。
    pumped: Duration,
    /// サーバーの `cmrt-standby-load: … event=finish` 行。
    finished: String,
    /// サーバーの `cmrt-standby-patch: … event=accepted` 行（受付応答）。
    accepted: String,
    /// サーバーの `cmrt-standby-patch: … event=completed` 行（完了通知）。
    completed: String,
}

/// **2 秒のロード中も、16 分音符の供給が止まらないこと。**
#[test]
#[ignore = "実機の play server 実行ファイルが要る（CMRT_TEST_PLAY_SERVER_EXE）"]
fn timeline_events_keep_reaching_the_server_while_a_standby_load_is_still_loading() {
    let exe = std::env::var(PLAY_SERVER_EXE_ENV).unwrap_or_else(|_| {
        panic!("{PLAY_SERVER_EXE_ENV} に play server の実行ファイルを渡すこと")
    });
    // 起動中の TUI（既定 62154）とも、他のテストのサーバーとも衝突させない。
    let port = pick_port(53_000);
    let server = TestPlayServer::spawn_with_env(
        &exe,
        port,
        INSTANCE_COUNT,
        &[(PATCH_LOAD_DELAY_ENV, LOAD_DELAY_MS.to_string())],
    );

    let cfg = crate::tests::cfg_for_port(port);
    let supervisor =
        crate::RealtimePlayServerSupervisor::with_live_instance_count(&cfg, INSTANCE_COUNT);
    supervisor
        .ensure_started_for_fast_midi()
        .expect("起動済みサーバーへ繋がらない");
    supervisor
        .set_connected_live_buffer_multiplier(BUFFER_MULTIPLIER)
        .expect("出力バッファ倍率を設定できない");

    let window = run_load_window(&supervisor, &server);
    window.report();
    window.assert_supply_never_stopped();

    // 完了したのだから、次の待機 instance の先読みを普通に始められる（不変条件 8）。
    let mut next = supervisor
        .begin_standby_patch(NEXT_STANDBY_INSTANCE, None)
        .expect("完了後なのに次の先読みを始められない");
    supervisor
        .poll_standby_patch(&mut next)
        .expect("次の先読みの poll が失敗した");
    supervisor.abandon_standby_patch(next);
}

/// timeline を張って刻み続け、途中で先読みを 1 件重ねる。
fn run_load_window(
    supervisor: &crate::RealtimePlayServerSupervisor,
    server: &TestPlayServer,
) -> LoadWindow {
    supervisor
        .begin_live_timeline(LiveTimelineConfig {
            timeline_id: TIMELINE_ID,
            sample_rate_hz: 48_000.0,
            tempo_bpm: TEMPO_BPM,
            time_signature_numerator: 4,
            time_signature_denominator: 4,
        })
        .expect("timeline を張れない");
    // timeline の原点。サーバー側の原点は BeginTimeline を処理した瞬間なので、
    // ここより僅かに前になる。lookahead はその差より十分大きく取ってある。
    let started = Instant::now();

    let mut accept = Duration::ZERO;
    let mut request = None;
    let mut baseline = None;
    let mut steps_during_load = 0;
    let mut max_send_during_load = Duration::ZERO;
    let mut completed = false;

    for step in 0..MAX_STEPS {
        sleep_until_send_time(started, step);
        let sending = Instant::now();
        supervisor
            .send_timeline_events(&step_events(step))
            .expect("timeline イベントを送れない");
        let send_elapsed = sending.elapsed();

        if step == WARMUP_STEPS {
            // 演奏が軌道に乗ってからの差分だけを見る。
            baseline = Some((
                supervisor.underrun_frames(),
                supervisor.timing_metrics().late_events_total,
            ));
            let began = Instant::now();
            request = Some(
                supervisor
                    .begin_standby_patch(STANDBY_INSTANCE, None)
                    .expect("待機 bank への先読みを受け付けてもらえない"),
            );
            accept = began.elapsed();
            continue;
        }

        if let Some(pending) = request.as_mut() {
            steps_during_load += 1;
            max_send_during_load = max_send_during_load.max(send_elapsed);
            if supervisor
                .poll_standby_patch(pending)
                .expect("先読みが失敗した")
                .is_some()
            {
                completed = true;
                // 確定済みの token を再 poll するとエラーになる。もう触らない。
                request = None;
            }
            continue;
        }
        // ロードが終わってもまだ刻む。サーバーの late は 5 秒ごとにしか publish
        // されないので、ロード区間を含む window が 1 回出るまで演奏を続けないと
        // 「遅れなかった」ことを確かめたつもりで 0 を読むだけになる。
        if completed && started.elapsed() >= PUMP_AT_LEAST {
            break;
        }
    }
    let pumped = started.elapsed();

    let (underrun_before, late_before) = baseline.expect("warmup まで刻めていない");
    assert!(
        completed,
        "{MAX_STEPS} ステップ刻んでも先読みの完了通知が届かない: {}",
        server.stderr_text()
    );
    // 完了通知はクライアントが先に見る（専用 slot への publish が先で、
    // サーバーの finish 行は同じ完了処理の中で出る）。行が揃うまで待つ。
    let finished = server.wait_for_stderr_line(|line| {
        line.starts_with(&format!(
            "cmrt-standby-load: bank=1 event=finish instance={STANDBY_INSTANCE}"
        ))
    });
    LoadWindow {
        accept,
        steps_during_load,
        max_send_during_load,
        underrun_before,
        underrun_after: supervisor.underrun_frames(),
        late_before,
        late_after: supervisor.timing_metrics().late_events_total,
        pumped,
        timeline_midi_between: count_lines_between(
            &server.stderr_snapshot(),
            |line| {
                line.starts_with(&format!(
                    "cmrt-standby-load: bank=1 event=start instance={STANDBY_INSTANCE}"
                ))
            },
            |line| {
                line.starts_with(&format!(
                    "cmrt-standby-load: bank=1 event=finish instance={STANDBY_INSTANCE}"
                ))
            },
            |line| line.starts_with("cmrt-ipc-recv: kind=timeline-midi"),
        ),
        accepted: server.wait_for_stderr_line(|line| {
            line.starts_with("cmrt-standby-patch: request=") && line.contains("event=accepted")
        }),
        completed: server.wait_for_stderr_line(|line| {
            line.starts_with("cmrt-standby-patch: request=") && line.contains("event=completed")
        }),
        finished,
    }
}

impl LoadWindow {
    /// 判定の前に実測値を全部出す。落ちたときに「何が起きたか」をログだけで追える。
    fn report(&self) {
        eprintln!(
            "timeline-during-preload: accept={:?} steps_during_load={} max_send={:?} \
             timeline_midi_between={} underrun {}→{} late {}→{} pumped={:?} | {} | {} | {}",
            self.accept,
            self.steps_during_load,
            self.max_send_during_load,
            self.timeline_midi_between,
            self.underrun_before,
            self.underrun_after,
            self.late_before,
            self.late_after,
            self.pumped,
            self.finished,
            self.accepted,
            self.completed,
        );
    }

    fn assert_supply_never_stopped(&self) {
        // 人工遅延が効いていること。効いていなければ以下は何も見ていない。
        assert!(
            self.finished.contains("result=ok"),
            "先読みが失敗した: {}",
            self.finished
        );
        assert!(
            number_field(&self.finished, "elapsed_ms") >= LOAD_DELAY_MS,
            "人工遅延が効いていない: {}",
            self.finished
        );

        // クライアント側が塞がれていないこと（旧設計の詰まり 1）。
        assert!(
            self.accept < CALL_BUDGET,
            "受付応答に {:?} かかった。ロード完了を待っている",
            self.accept
        );
        assert!(
            self.steps_during_load >= STEPS_DURING_LOAD_AT_LEAST,
            "ロード中に {} ステップしか送れていない",
            self.steps_during_load
        );
        // 実障害の `sender_queue_max_us=2928489` に当たる値。ここでは grid の
        // 送信スレッドを通していないので、1 回の送信呼び出しに掛かった最大時間で見る。
        assert!(
            self.max_send_during_load < CALL_BUDGET,
            "ロード中の送信 1 回に {:?} 掛かった。IPC が詰まっている",
            self.max_send_during_load
        );

        // サーバー側が塞がれていないこと（旧設計の詰まり 2）。
        // ロード完了後にまとめて届いたのでは、この数は 0 のままになる。
        assert!(
            self.timeline_midi_between >= STEPS_DURING_LOAD_AT_LEAST,
            "ロードの開始と終了の間に timeline-midi の受信が {} 行しか無い",
            self.timeline_midi_between
        );

        // ロード中も演奏 bank が回っていたこと。
        assert!(
            number_field(&self.finished, "blocks_elsewhere") > 0,
            "ロード中に演奏 bank が 1 ブロックも進んでいない: {}",
            self.finished
        );
        assert_eq!(
            number_field(&self.finished, "underrun_frames"),
            0,
            "ロード中に出力が途切れた: {}",
            self.finished
        );
        assert_eq!(
            self.underrun_after, self.underrun_before,
            "先読みの前後で underrun が増えた"
        );
        // late を読む意味があること。publish を跨いでいなければ、次の assert は
        // 0 と 0 を比べるだけで何も見ていない。
        assert!(
            self.pumped >= METRICS_WINDOW,
            "timing metrics の publish 周期（{METRICS_WINDOW:?}）を跨いでいない: {:?}",
            self.pumped
        );
        // 本題。供給が止まれば、発音時刻を過ぎたイベントが遅れて届いてここが跳ねる。
        assert_eq!(
            self.late_after, self.late_before,
            "先読み中にイベントが遅れた（実障害では late が 260 増えた）"
        );

        // v10 の 2 段階契約が、後からログだけで辿れること。受付（generic ACK）と
        // 完了（専用 slot）は別の時点で出るので、同じ request ID で結べないと
        // 「どの先読みが何秒掛かったか」が診断できない。
        assert_eq!(
            number_field(&self.accepted, "request"),
            number_field(&self.completed, "request"),
            "受付と完了の request ID が違う: {} / {}",
            self.accepted,
            self.completed
        );
        assert!(
            self.completed.contains("result=ok"),
            "完了通知が成功として publish されていない: {}",
            self.completed
        );
    }
}

/// `step` ぶんの送信時刻まで眠る。**発音時刻の [`LOOKAHEAD`] 前**に送る。
fn sleep_until_send_time(started: Instant, step: usize) {
    let due = Duration::from_secs_f64(step as f64 * STEP_SECONDS).saturating_sub(LOOKAHEAD);
    let elapsed = started.elapsed();
    if due > elapsed {
        std::thread::sleep(due - elapsed);
    }
}

/// 16 分音符 1 ステップぶんの note on / note off。bank 0 の 2 instance が鳴る。
fn step_events(step: usize) -> Vec<TimelineMidiEvent> {
    let at = step as f64 * STEP_SECONDS;
    let mut events = Vec::new();
    for instance_id in 0..2u8 {
        let key = 60 + (step % 12) as u8 + instance_id * 12;
        events.push(TimelineMidiEvent {
            timeline_id: TIMELINE_ID,
            instance_id,
            timeline_seconds: at,
            message: [0x90, key, 100],
        });
        events.push(TimelineMidiEvent {
            timeline_id: TIMELINE_ID,
            instance_id,
            // 次のステップへ食い込ませない。
            timeline_seconds: at + STEP_SECONDS * 0.9,
            message: [0x80, key, 0],
        });
    }
    events
}
