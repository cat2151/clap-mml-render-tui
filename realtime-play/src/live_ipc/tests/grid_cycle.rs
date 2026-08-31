//! grid の 1 周ぶんの timeline を **TUI 無しで**流し、先読みの前後で
//! underrun / late event が悪化しないことを見る（受け入れ条件 9 の機械化）。
//!
//! # なぜ TUI を操作しなくてよいか
//! 受け入れ条件 9 が見たいのは「preload 前後の metrics に退行がないこと」であって、
//! 画面でも耳でもない。metrics は SHM 越しにクライアントから読める
//! （[`RealtimePlayServerSupervisor::underrun_frames`] /
//! [`RealtimePlayServerSupervisor::timing_metrics`]）ので、grid sequencer が送るのと
//! 同じ 3 つ（`begin_live_timeline` → `send_timeline_events` → `prepare_standby_patch`）を
//! ここから直接送れば、TUI を起こす必要がない。
//!
//! # 人工遅延を入れる理由
//! 既定音色の先読みは 1ms 未満で終わるので、そのままでは「止まらないこと」を
//! 確かめる窓が開かない。サーバー側のテスト注入点（`CMRT_TEST_PATCH_LOAD_DELAY_MS`）で
//! 1 件あたり [`PRELOAD_DELAY_MS`] だけ止め、**実際に patch load が長い状況**を作る。
//!
//! # 出力バッファの厚さを 2 通り走らせる理由
//! grid sequencer は「先読み中はサーバーのレンダースレッドが止まる」前提で、
//! preload のあいだだけ出力バッファ倍率を 16 まで厚くしていた（Stage 5 で外した
//! 暫定回避）。倍率は**クライアントが決める値**なのでサーバーの環境変数では
//! 切り替えられず、ここから明示的に指定する。薄い側（[`THIN_BUFFER_MULTIPLIER`]）が
//! 通ることが、暫定回避を外してよい根拠そのものになる。
//!
//! # これが**番人にならない**こと
//! ここは 1 周ぶんのイベントを**先読みの前に全部**送ってしまう。ロード中に
//! timeline の供給が止まっても、送り終わったイベントで演奏が続くので metrics は
//! 悪化しない。2026-08-31 の supply starvation（ロード中に MIDI が届かず note off が
//! 遅れる）はここでは検出できない。その番人は `timeline_during_preload.rs`。
//!
//! late の読みについても同じ注意が要る。サーバーは timing metrics を 5 秒周期でしか
//! publish しない（play-server の `player/timing_diagnostics.rs` の `WINDOW`）。
//! この test は 2 秒ほどで終わるので、`late_before` / `late_after` はどちらも 0 の
//! ままになりやすい。ここで意味のある回帰判定は underrun 側だと考えること。
//!
//! ```text
//! $env:CMRT_TEST_PLAY_SERVER_EXE = "...\clap-mml-realtime-play-server.exe"
//! cargo test -p cmrt-realtime-play -- --include-ignored grid_cycle
//! ```

use std::time::Duration;

use crate::{LiveTimelineConfig, TimelineMidiEvent};

use super::harness::{
    number_field, pick_port, TestPlayServer, PATCH_LOAD_DELAY_ENV, PLAY_SERVER_EXE_ENV,
};

/// 2 track × 2 bank。grid の最小構成で、bank 境界も preload も成立する。
const INSTANCE_COUNT: usize = 4;
const TIMELINE_ID: u64 = 1;
const TEMPO_BPM: f64 = 130.0;
/// 1 周のステップ数と 1 ステップの長さ（8 分音符）。
const STEPS: usize = 8;
const STEP_SECONDS: f64 = 30.0 / TEMPO_BPM;
/// 先読み 1 件あたりの人工ロード時間。2 件で 400ms、1 周（約 1.8 秒）に収まる。
const PRELOAD_DELAY_MS: u64 = 200;
/// 演奏が軌道に乗るまで待つ時間。ここまでの metrics を基準にする。
const WARMUP: Duration = Duration::from_millis(400);

/// grid が実際に使う厚さ（`grid-sequencer` の `INITIAL_BUFFER_MULTIPLIER`）。
/// 512 フレーム ÷ 48kHz × 2 ≒ 21ms しか余裕がないので、レンダーが 21ms でも
/// 止まれば underrun になる。**Stage 5 で暫定回避を外したあとの実運用値。**
const THIN_BUFFER_MULTIPLIER: u16 = 2;
/// Stage 5 で外した暫定回避の厚さ（≒170ms）。外す前との比較用に残してある。
const BOOSTED_BUFFER_MULTIPLIER: u16 = 16;

/// **grid 1 周の演奏中に待機 bank を先読みしても、underrun と late event が増えないこと。**
///
/// 受け入れ条件 9。ここが緑なら、実機で TUI を操作して確かめていた
/// 「preload 前後の metrics に退行がない」は機械で判定できている。
///
/// 厚さは grid が実運用で使う薄い側。**暫定回避（倍率 16）が無くても通ることが、
/// Stage 5 でそれを削除してよい根拠。**
#[test]
#[ignore = "実機の play server 実行ファイルが要る（CMRT_TEST_PLAY_SERVER_EXE）"]
fn a_standby_preload_does_not_regress_the_metrics_of_a_running_grid_cycle() {
    let port = pick_port(49_000);
    run_grid_cycle_with_preload(port, THIN_BUFFER_MULTIPLIER).assert_no_regression();
}

/// 同じことを、**Stage 5 で外した暫定回避の厚さ**でも確かめる。
///
/// 薄い側だけが緑でも「厚くすると壊れる」ことは無いと言い切れないので、
/// 削除の前後を同じ物差しで比べられるよう両方を残す。
#[test]
#[ignore = "実機の play server 実行ファイルが要る（CMRT_TEST_PLAY_SERVER_EXE）"]
fn a_standby_preload_does_not_regress_the_metrics_with_the_old_preload_buffer_boost() {
    let port = pick_port(50_000);
    run_grid_cycle_with_preload(port, BOOSTED_BUFFER_MULTIPLIER).assert_no_regression();
}

/// 先読みを挟んだ 1 周ぶんの実測値。数値は判定に使うだけでなく、
/// `--nocapture` で見て資料へ残せるように stderr へも出す。
struct CycleOutcome {
    buffer_multiplier: u16,
    underrun_before: u64,
    underrun_after: u64,
    late_before: u64,
    late_after: u64,
    /// サーバーの `cmrt-standby-load: … event=finish` 行（最後の 1 件ぶん）。
    finished: String,
}

impl CycleOutcome {
    fn assert_no_regression(&self) {
        eprintln!(
            "grid-cycle: buffer_multiplier={} underrun {}→{} late {}→{} | {}",
            self.buffer_multiplier,
            self.underrun_before,
            self.underrun_after,
            self.late_before,
            self.late_after,
            self.finished,
        );
        assert_eq!(
            self.underrun_after, self.underrun_before,
            "先読み中に出力が途切れた（buffer_multiplier={} underrun frames {} → {}）",
            self.buffer_multiplier, self.underrun_before, self.underrun_after
        );
        assert_eq!(
            self.late_after, self.late_before,
            "先読み中にイベントが遅れた（buffer_multiplier={} late events {} → {}）",
            self.buffer_multiplier, self.late_before, self.late_after
        );
        // 先読みが本当に止まっていたこと、その間も演奏 bank が回っていたこと。
        // metrics が「変わらなかった」だけだと、そもそも何も起きていない可能性を消せない。
        assert!(self.finished.contains("result=ok"), "{}", self.finished);
        assert!(
            number_field(&self.finished, "elapsed_ms") >= PRELOAD_DELAY_MS,
            "人工遅延が効いていない: {}",
            self.finished
        );
        assert!(
            number_field(&self.finished, "blocks_elsewhere") >= 10,
            "先読み中に演奏 bank が進んでいない: {}",
            self.finished
        );
    }
}

/// grid 1 周ぶんを流しながら待機 bank へ 2 件先読みし、前後の metrics を返す。
///
/// `buffer_multiplier` は**クライアントが決める出力バッファの厚さ**。grid の
/// `set_connected_live_buffer_multiplier()` と同じ口を叩く。
fn run_grid_cycle_with_preload(port: u16, buffer_multiplier: u16) -> CycleOutcome {
    let exe = std::env::var(PLAY_SERVER_EXE_ENV).unwrap_or_else(|_| {
        panic!("{PLAY_SERVER_EXE_ENV} に play server の実行ファイルを渡すこと")
    });
    let server = TestPlayServer::spawn_with_env(
        &exe,
        port,
        INSTANCE_COUNT,
        &[(PATCH_LOAD_DELAY_ENV, PRELOAD_DELAY_MS.to_string())],
    );

    let cfg = crate::tests::cfg_for_port(port);
    let supervisor =
        crate::RealtimePlayServerSupervisor::with_live_instance_count(&cfg, INSTANCE_COUNT);
    supervisor
        .ensure_started_for_fast_midi()
        .expect("起動済みサーバーへ繋がらない");
    supervisor
        .set_connected_live_buffer_multiplier(buffer_multiplier)
        .expect("出力バッファ倍率を設定できない");

    // grid と同じ順序: timeline を張って、1 周ぶんのイベントを先に積む。
    supervisor
        .begin_live_timeline(LiveTimelineConfig {
            timeline_id: TIMELINE_ID,
            sample_rate_hz: 48_000.0,
            tempo_bpm: TEMPO_BPM,
            time_signature_numerator: 4,
            time_signature_denominator: 4,
        })
        .expect("timeline を張れない");
    supervisor
        .send_timeline_events(&one_cycle())
        .expect("1 周ぶんの timeline イベントを送れない");
    server.wait_for_stderr_line(|line| line.starts_with("cmrt-bank-render: bank=0 "));
    std::thread::sleep(WARMUP);

    // 先読み前の基準。演奏はこの後もずっと続いている。
    let underrun_before = supervisor.underrun_frames();
    let late_before = supervisor.timing_metrics().late_events_total;

    // 待機 bank（instance 2, 3）へ 1 件ずつ先読みする。
    // v10 以降、grid 本体は非同期 API（begin/poll）を使う。ここが同期 wrapper のままなのは、
    // このテストが見るのが「先読み中に underrun / late が増えないこと」だけで、
    // 供給が止まらないことの番人は timeline_during_preload.rs だからである。
    for instance in 2..INSTANCE_COUNT as u8 {
        supervisor
            .prepare_standby_patch(instance, None)
            .expect("待機 bank への先読みが失敗した");
    }

    let underrun_after = supervisor.underrun_frames();
    let late_after = supervisor.timing_metrics().late_events_total;
    let finished = server.wait_for_stderr_line(|line| {
        line.starts_with("cmrt-standby-load: bank=1 event=finish instance=3")
    });
    CycleOutcome {
        buffer_multiplier,
        underrun_before,
        underrun_after,
        late_before,
        late_after,
        finished,
    }
}

/// grid 1 周ぶんの note on / note off。bank 0 の 2 instance（= 2 track）が鳴る。
fn one_cycle() -> Vec<TimelineMidiEvent> {
    let mut events = Vec::new();
    for step in 0..STEPS {
        let at = step as f64 * STEP_SECONDS;
        for instance_id in 0..2u8 {
            let key = 60 + step as u8 + instance_id * 12;
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
    }
    events
}
