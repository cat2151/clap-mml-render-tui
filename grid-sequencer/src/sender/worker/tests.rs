//! コマンドループが「先読みのロード中も止まらない」ことを、実サーバー無しで固定する。
//!
//! ここで使う [`FakeBackend`] は完了通知を**明示的に解放するまで返さない**。旧設計
//! （`Preload` 分岐でロード完了を同期で待つ）なら、解放前に送った `Send` は 1 件も
//! backend へ届かず、[`Observed::wait_for`] が時間切れで panic する。

use std::{
    collections::HashMap,
    sync::{
        atomic::{AtomicU64, Ordering},
        mpsc, Arc, Condvar, Mutex,
    },
    thread::JoinHandle,
    time::{Duration, Instant},
};

use cmrt_realtime_play::TimelineMidiEvent;

use crate::sender::gain_summary::describe_adjusted;

use super::{
    preload::PreloadOutcome, run_command_loop, GridMidiCommand, GridSenderBackend,
    PreloadGeneration,
};

/// 観測を待つ上限。到達しなければ「ループが塞がっている」ので panic させる。
/// ループの `recv_timeout` は 50ms なので、これは充分な余裕。
const WAIT_LIMIT: Duration = Duration::from_secs(5);

#[test]
fn adjusted_rows_are_named_with_one_based_numbers() {
    assert_eq!(describe_adjusted(&[1.0, 1.0, 1.0]), "none");
    assert_eq!(describe_adjusted(&[0.0, 1.0, 1.0]), "row1:mute");
    assert_eq!(describe_adjusted(&[1.0, 0.5, 2.0]), "row2:0.50x,row3:2.00x");
}

#[test]
fn timeline_events_keep_flowing_while_a_preload_is_still_loading() {
    let loop_under_test = LoopUnderTest::start();
    loop_under_test.preload(1, 3);
    loop_under_test
        .observed
        .wait_for(|state| state.begun == [1]);

    // 完了は解放しないまま、次のステップぶんのイベントを投げ続ける。
    for _ in 0..3 {
        loop_under_test.send_events();
    }
    loop_under_test.observed.wait_for(|state| state.sends == 3);

    // ここで進捗が進んでいたら、受付を完了と取り違えている。
    assert_eq!(loop_under_test.observed.snapshot().outcomes, Vec::new());

    loop_under_test.observed.release(1, Ok(()));
    loop_under_test
        .observed
        .wait_for(|state| state.outcomes.len() == 1);
    assert_eq!(
        loop_under_test.observed.snapshot().outcomes,
        vec![RecordedOutcome {
            instance_id: 3,
            request_id: Some(1),
            error: None,
            stale: false,
        }]
    );
    loop_under_test.finish();
}

#[test]
fn a_completion_from_a_cancelled_cycle_does_not_advance_the_next_cycle() {
    let loop_under_test = LoopUnderTest::start();
    loop_under_test.preload(1, 0);
    loop_under_test
        .observed
        .wait_for(|state| state.begun == [1]);

    // `finish_preload` / `stop` 相当。要求は wire 上に残るが、もう自分のものではない。
    loop_under_test.generation.fetch_add(1, Ordering::SeqCst);
    loop_under_test.observed.release(1, Ok(()));
    loop_under_test
        .observed
        .wait_for(|state| state.outcomes.len() == 1);
    assert!(loop_under_test.observed.snapshot().outcomes[0].stale);

    // 次のサイクル。世代が合っている要求だけが進捗になる。
    let generation = loop_under_test.generation.fetch_add(1, Ordering::SeqCst) + 1;
    loop_under_test.preload(generation, 5);
    loop_under_test
        .observed
        .wait_for(|state| state.begun == [1, 2]);
    loop_under_test.observed.release(2, Ok(()));
    loop_under_test
        .observed
        .wait_for(|state| state.outcomes.len() == 2);
    assert_eq!(
        loop_under_test.observed.snapshot().outcomes[1],
        RecordedOutcome {
            instance_id: 5,
            request_id: Some(2),
            error: None,
            stale: false,
        }
    );
    loop_under_test.finish();
}

#[test]
fn a_preload_queued_behind_a_cancelled_one_is_never_sent_to_the_server() {
    let loop_under_test = LoopUnderTest::start();
    loop_under_test.preload(1, 0);
    loop_under_test
        .observed
        .wait_for(|state| state.begun == [1]);
    // 完了 slot は 1 件ぶんしか無いので、2 件目は受付の順番待ちになる。
    loop_under_test.preload(1, 4);

    loop_under_test.generation.fetch_add(1, Ordering::SeqCst);
    loop_under_test.observed.release(1, Ok(()));
    loop_under_test
        .observed
        .wait_for(|state| state.outcomes.len() == 2);
    let state = loop_under_test.observed.snapshot();
    assert_eq!(state.begun, [1], "順番待ちは受付にも出さない");
    assert!(state.outcomes.iter().all(|outcome| outcome.stale));
    loop_under_test.finish();
}

#[test]
fn stop_hands_a_pending_preload_back_without_waiting_for_the_load() {
    let loop_under_test = LoopUnderTest::start();
    loop_under_test.preload(1, 2);
    loop_under_test
        .observed
        .wait_for(|state| state.begun == [1]);

    loop_under_test.send(GridMidiCommand::Stop);
    loop_under_test
        .observed
        .wait_for(|state| state.slow_commands == ["stop"]);
    let state = loop_under_test.observed.snapshot();
    assert_eq!(state.abandoned, [1]);
    assert_eq!(
        state.outcomes,
        vec![RecordedOutcome {
            instance_id: 2,
            request_id: Some(1),
            error: None,
            stale: true,
        }],
        "打ち切りは失敗として数えない"
    );
    loop_under_test.finish();
}

#[test]
fn a_rejected_preload_is_reported_as_a_failure_of_the_current_cycle() {
    let loop_under_test = LoopUnderTest::start();
    loop_under_test.observed.state.lock().unwrap().begin_error =
        Some("standby patch load 7 is still in flight".to_string());
    loop_under_test.preload(1, 6);
    loop_under_test
        .observed
        .wait_for(|state| state.outcomes.len() == 1);
    let mut snapshot = loop_under_test.observed.snapshot();
    let outcome = snapshot.outcomes.remove(0);
    assert_eq!(outcome.instance_id, 6);
    assert_eq!(outcome.request_id, None);
    assert!(!outcome.stale, "受付の失敗は今のサイクルの失敗");
    assert!(outcome
        .error
        .unwrap()
        .contains("standby patch load 7 is still in flight"));
    loop_under_test.finish();
}

/// 本物の [`run_command_loop`] を別スレッドで回し、fake backend の観測を待つ足場。
struct LoopUnderTest {
    tx: mpsc::Sender<GridMidiCommand>,
    observed: Arc<Observed>,
    generation: PreloadGeneration,
    worker: Option<JoinHandle<()>>,
}

impl LoopUnderTest {
    fn start() -> Self {
        let (tx, rx) = mpsc::channel();
        let observed = Arc::new(Observed::default());
        let generation: PreloadGeneration = Arc::new(AtomicU64::new(1));
        let worker_observed = Arc::clone(&observed);
        let worker_generation = Arc::clone(&generation);
        let worker = std::thread::spawn(move || {
            let mut backend = FakeBackend {
                observed: worker_observed,
            };
            run_command_loop(rx, &mut backend, &worker_generation);
        });
        Self {
            tx,
            observed,
            generation,
            worker: Some(worker),
        }
    }

    fn send(&self, command: GridMidiCommand) {
        self.tx.send(command).expect("the command loop is running");
    }

    fn preload(&self, generation: u64, instance_id: u8) {
        self.send(GridMidiCommand::Preload {
            generation,
            instance_id,
            patch: Some(format!("patch-{instance_id}")),
        });
    }

    fn send_events(&self) {
        self.send(GridMidiCommand::Send {
            events: vec![TimelineMidiEvent {
                timeline_id: 1,
                instance_id: 0,
                timeline_seconds: 0.0,
                message: [0x90, 60, 100],
            }],
            queued_at: Instant::now(),
            pump_lateness: Duration::ZERO,
        });
    }

    fn finish(mut self) {
        let worker = self.worker.take().expect("the worker is still owned");
        self.send(GridMidiCommand::Shutdown);
        worker.join().expect("the command loop should exit cleanly");
    }
}

impl Drop for LoopUnderTest {
    fn drop(&mut self) {
        if let Some(worker) = self.worker.take() {
            let _ = self.tx.send(GridMidiCommand::Shutdown);
            let _ = worker.join();
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct RecordedOutcome {
    instance_id: u8,
    request_id: Option<u32>,
    error: Option<String>,
    stale: bool,
}

#[derive(Default)]
struct State {
    sends: usize,
    begun: Vec<u32>,
    abandoned: Vec<u32>,
    released: HashMap<u32, Result<(), String>>,
    outcomes: Vec<RecordedOutcome>,
    slow_commands: Vec<&'static str>,
    begin_error: Option<String>,
    next_request_id: u32,
}

/// テストスレッドとループスレッドが共有する観測。`Condvar` で待つので、
/// 「たぶんもう進んだ」という sleep に頼らない。
#[derive(Default)]
struct Observed {
    state: Mutex<State>,
    changed: Condvar,
}

impl Observed {
    fn snapshot(&self) -> State {
        let state = self.state.lock().unwrap();
        State {
            sends: state.sends,
            begun: state.begun.clone(),
            abandoned: state.abandoned.clone(),
            released: state.released.clone(),
            outcomes: state.outcomes.clone(),
            slow_commands: state.slow_commands.clone(),
            begin_error: state.begin_error.clone(),
            next_request_id: state.next_request_id,
        }
    }

    fn wait_for(&self, mut condition: impl FnMut(&State) -> bool) {
        let deadline = Instant::now() + WAIT_LIMIT;
        let mut state = self.state.lock().unwrap();
        while !condition(&state) {
            let remaining = deadline.saturating_duration_since(Instant::now());
            assert!(
                !remaining.is_zero(),
                "コマンドループが期待した状態へ進まなかった（塞がっている疑い）"
            );
            let (next, _) = self.changed.wait_timeout(state, remaining).unwrap();
            state = next;
        }
    }

    /// ロードが終わったことにする。ここを呼ぶまで完了通知は返らない。
    fn release(&self, request_id: u32, result: Result<(), String>) {
        self.state
            .lock()
            .unwrap()
            .released
            .insert(request_id, result);
        self.changed.notify_all();
    }
}

struct FakeBackend {
    observed: Arc<Observed>,
}

impl GridSenderBackend for FakeBackend {
    type Standby = u32;

    fn send_timeline(
        &mut self,
        _events: &[TimelineMidiEvent],
        _queued_at: Instant,
        _pump_lateness: Duration,
    ) {
        self.observed.state.lock().unwrap().sends += 1;
        self.observed.changed.notify_all();
    }

    fn begin_standby(
        &mut self,
        _instance_id: u8,
        _patch: Option<&str>,
    ) -> anyhow::Result<Self::Standby> {
        let mut state = self.observed.state.lock().unwrap();
        if let Some(error) = state.begin_error.clone() {
            return Err(anyhow::anyhow!(error));
        }
        state.next_request_id += 1;
        let request_id = state.next_request_id;
        state.begun.push(request_id);
        drop(state);
        self.observed.changed.notify_all();
        Ok(request_id)
    }

    fn poll_standby(&mut self, request: &mut Self::Standby) -> anyhow::Result<Option<()>> {
        match self.observed.state.lock().unwrap().released.get(request) {
            None => Ok(None),
            Some(Ok(())) => Ok(Some(())),
            Some(Err(error)) => Err(anyhow::anyhow!(error.clone())),
        }
    }

    fn abandon_standby(&mut self, request: Self::Standby) {
        self.observed.state.lock().unwrap().abandoned.push(request);
        self.observed.changed.notify_all();
    }

    fn standby_request_id(&self, request: &Self::Standby) -> u32 {
        *request
    }

    fn record_preload_outcome(&mut self, outcome: PreloadOutcome) {
        self.observed
            .state
            .lock()
            .unwrap()
            .outcomes
            .push(RecordedOutcome {
                instance_id: outcome.instance_id,
                request_id: outcome.request_id,
                error: outcome.error,
                stale: outcome.stale,
            });
        self.observed.changed.notify_all();
    }

    fn handle_slow_command(&mut self, command: GridMidiCommand) {
        let name = match command {
            GridMidiCommand::StartServer => "start-server",
            GridMidiCommand::BeginTimeline { .. } => "begin-timeline",
            GridMidiCommand::SetLiveTempo { .. } => "set-live-tempo",
            GridMidiCommand::Prepare { .. } => "prepare",
            GridMidiCommand::SetRowPatch { .. } => "set-row-patch",
            GridMidiCommand::SetGains { .. } => "set-gains",
            GridMidiCommand::SetAutoGain { .. } => "set-auto-gain",
            GridMidiCommand::Stop => "stop",
            GridMidiCommand::Send { .. }
            | GridMidiCommand::Preload { .. }
            | GridMidiCommand::Shutdown => unreachable!("the loop handles these itself"),
        };
        self.observed.state.lock().unwrap().slow_commands.push(name);
        self.observed.changed.notify_all();
    }

    fn poll_runtime(&mut self, _now: Instant) {}
}
