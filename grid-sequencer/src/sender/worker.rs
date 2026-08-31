//! MIDI 送信ワーカースレッドのコマンドループ。
//!
//! [`super::GridMidiSender`] が投げたコマンドを1本のスレッドで順に処理し、
//! 空き時間に出力バッファの適応調整とリミッターメーターの取り込みを行う。
//! サーバーとやり取りするのはこのスレッドだけで、UI スレッドは status を読むだけ。
//!
//! # 塞いではいけない経路
//!
//! このループが止まると timeline event の送出も止まる。止めてよいのは
//! 「その間どのみち音が出ない」コマンド（起動・全 instance のロード・停止）だけで、
//! **演奏中に走る先読み（[`GridMidiCommand::Preload`]）で待ってはいけない**。
//! v9 まではここで patch load の完了を同期で待っており、約 3 秒のロード中は
//! `Send` が mpsc に溜まりっぱなしになって note-off が遅れていた。
//!
//! そのため、ループが直接扱うのは次の3つだけ:
//!
//! - `Send`: そのまま backend へ流す
//! - `Preload`: [`PreloadTracker`] へ預けて即座に戻る
//! - `Stop` / `Shutdown`: 先読みを畳んでから backend へ
//!
//! 残りは [`GridSenderBackend::handle_slow_command`] へ委ねる。実サーバー向けの
//! 実装は [`supervisor_backend`]。この分け方のおかげで、ループ自体は
//! 実サーバー無しの fake backend で試験できる（[`tests`]）。

use std::{
    sync::{atomic::Ordering, mpsc, Arc, Mutex},
    time::{Duration, Instant},
};

use cmrt_realtime_play::{RealtimePlayServerSupervisor, TimelineMidiEvent};

use super::{GridConnectionStatus, GridMidiCommand, PreloadGeneration};

mod preload;
mod runtime_poll;
mod supervisor_backend;

use preload::{PreloadOutcome, PreloadTracker};
use supervisor_backend::SupervisorBackend;

const METER_POLL_INTERVAL: Duration = Duration::from_millis(50);

/// コマンドループがサーバーへ触るための窓口。
///
/// 「止めてはいけない経路」（timeline 送出と先読みの受付/完了ポーリング）だけを
/// 個別のメソッドにし、残りは [`Self::handle_slow_command`] へまとめてある。
/// 全メソッドを機械的に列挙した trait にしないのは、ループが守るべき境界そのものを
/// 型で表すため。
pub(super) trait GridSenderBackend {
    /// 先読み1件の受付票。実装では `cmrt_realtime_play::StandbyPatchRequest`。
    type Standby;

    fn send_timeline(
        &mut self,
        events: &[TimelineMidiEvent],
        queued_at: Instant,
        pump_lateness: Duration,
    );

    /// 先読みを要求し、**受付までで**戻る。ロードの完了は待たない。
    fn begin_standby(
        &mut self,
        instance_id: u8,
        patch: Option<&str>,
    ) -> anyhow::Result<Self::Standby>;

    /// 完了通知を非 blocking に読む。`Ok(None)` はまだロード中。
    fn poll_standby(&mut self, request: &mut Self::Standby) -> anyhow::Result<Option<()>>;

    /// 結果を捨てて、この要求を「自分のもの」ではなくする。
    fn abandon_standby(&mut self, request: Self::Standby);

    fn standby_request_id(&self, request: &Self::Standby) -> u32;

    /// 先読み1件の決着をログと status へ書く。
    fn record_preload_outcome(&mut self, outcome: PreloadOutcome);

    /// 完了まで待ってよいコマンド。待っている間 timeline event は送れない。
    fn handle_slow_command(&mut self, command: GridMidiCommand);

    /// コマンドの空き時間にサーバー状態を取り込む。
    fn poll_runtime(&mut self, now: Instant);
}

pub(super) fn run_midi_sender(
    rx: mpsc::Receiver<GridMidiCommand>,
    supervisor: Arc<RealtimePlayServerSupervisor>,
    status: Arc<Mutex<GridConnectionStatus>>,
    preload_generation: PreloadGeneration,
) {
    let mut backend = SupervisorBackend::new(supervisor, status);
    run_command_loop(rx, &mut backend, &preload_generation);
}

pub(super) fn run_command_loop<B: GridSenderBackend>(
    rx: mpsc::Receiver<GridMidiCommand>,
    backend: &mut B,
    preload_generation: &PreloadGeneration,
) {
    let mut preload = PreloadTracker::new();
    loop {
        // 完了通知を見るのは**毎周回の先頭**。「コマンドが来ない暇なときだけ」に
        // すると、イベント送信が続いている間ずっと先読みの完了に気づけない。
        let outcomes = preload.advance(backend, preload_generation.load(Ordering::SeqCst));
        report_preload(backend, outcomes);
        let command = match rx.recv_timeout(METER_POLL_INTERVAL) {
            Ok(command) => command,
            Err(mpsc::RecvTimeoutError::Timeout) => {
                backend.poll_runtime(Instant::now());
                continue;
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        };
        match command {
            GridMidiCommand::Send {
                events,
                queued_at,
                pump_lateness,
            } => backend.send_timeline(&events, queued_at, pump_lateness),
            GridMidiCommand::Preload {
                generation,
                instance_id,
                patch,
            } => {
                // 先読み専用コマンド。宛先は必ず「発音 deadline を越えて非演奏に
                // なった待機 bank」なので、サーバーはその bank を止めてロードしてよい。
                // 現在 bank を触る `SetRowPatch` とは別の API であることが契約の本体。
                //
                // **ここで出力バッファを厚くしないこと。** 演奏 bank の render は
                // サーバー側の bank worker が続けている。厚くしても underrun は減らず、
                // 発音の遅れが増えるだけ。実測（`realtime-play/src/live_ipc/tests/
                // grid_cycle.rs`）でも倍率 2 と 16 の両方で underrun / late の増分は 0。
                let outcomes = preload.submit(
                    backend,
                    generation,
                    preload_generation.load(Ordering::SeqCst),
                    instance_id,
                    patch,
                );
                report_preload(backend, outcomes);
            }
            GridMidiCommand::Stop => {
                let outcomes = preload.cancel(backend);
                report_preload(backend, outcomes);
                backend.handle_slow_command(GridMidiCommand::Stop);
            }
            GridMidiCommand::Shutdown => {
                let outcomes = preload.cancel(backend);
                report_preload(backend, outcomes);
                break;
            }
            slow => backend.handle_slow_command(slow),
        }
    }
}

fn report_preload<B: GridSenderBackend>(backend: &mut B, outcomes: Vec<PreloadOutcome>) {
    for outcome in outcomes {
        backend.record_preload_outcome(outcome);
    }
}

#[cfg(test)]
mod tests;
