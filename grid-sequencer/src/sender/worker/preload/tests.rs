//! 世代の TOCTOU を**乱数にも並列にも頼らず**固定する。
//!
//! 本物の穴は「世代を読む」と「完了を観測する」の間に UI スレッドの `fetch_add` が
//! 挟まった回だけ踏む。実機では隙間が数ナノ秒なので、狙って再現させる手が無い
//! （`sender::worker::tests` はスレッドを 2 本走らせてこの隙間へ偶然割り込んでおり、
//! だから flaky だった）。
//!
//! ここでは fake backend の `poll_standby` が**完了を返すのと同時に世代を上げる**。
//! これは「隙間のいちばん奥で畳まれた回」そのもので、スレッドを 1 本も増やさずに
//! 決定的に再現できる。

use std::sync::{atomic::AtomicU64, Arc};

use super::*;
use crate::sender::GridMidiCommand;

/// 完了を返す瞬間に世代を上げる fake。`fold_on_poll` が「隙間のいちばん奥」を表す。
struct FoldingBackend {
    generation: PreloadGeneration,
    /// 次に `poll_standby` が完了を返すとき、その場で世代を 1 つ上げる。
    fold_on_poll: bool,
    begun: Vec<u8>,
    next_request_id: u32,
}

impl FoldingBackend {
    fn new(generation: &PreloadGeneration) -> Self {
        Self {
            generation: Arc::clone(generation),
            fold_on_poll: false,
            begun: Vec::new(),
            next_request_id: 0,
        }
    }
}

impl GridSenderBackend for FoldingBackend {
    type Standby = u32;

    fn send_timeline(
        &mut self,
        _events: &[cmrt_realtime_play::TimelineMidiEvent],
        _queued_at: Instant,
        _pump_lateness: Duration,
    ) {
        unreachable!("this fake only exercises the preload tracker");
    }

    fn begin_standby(
        &mut self,
        instance_id: u8,
        _patch: Option<&str>,
    ) -> anyhow::Result<Self::Standby> {
        self.next_request_id += 1;
        self.begun.push(instance_id);
        Ok(self.next_request_id)
    }

    fn poll_standby(&mut self, _request: &mut Self::Standby) -> anyhow::Result<Option<()>> {
        if self.fold_on_poll {
            // 完了を観測させる「その瞬間」にサイクルを畳む。`r` キー・画面離脱・
            // live edit で UI スレッドがやることと同じ。
            self.fold_on_poll = false;
            self.generation.fetch_add(1, Ordering::SeqCst);
        }
        Ok(Some(()))
    }

    fn abandon_standby(&mut self, _request: Self::Standby) {}

    fn standby_request_id(&self, request: &Self::Standby) -> u32 {
        *request
    }

    fn record_preload_outcome(&mut self, _outcome: PreloadOutcome) {
        unreachable!("the tracker returns outcomes; the loop is what records them");
    }

    fn handle_slow_command(&mut self, _command: GridMidiCommand) {
        unreachable!("this fake only exercises the preload tracker");
    }

    fn poll_runtime(&mut self, _now: Instant) {}
}

/// 受付だけ済ませて「まだロード中」で止めた tracker を作る。
fn tracker_with_one_in_flight(
    generation: &PreloadGeneration,
    backend: &mut FoldingBackend,
    instance_id: u8,
) -> PreloadTracker<u32> {
    let mut tracker = PreloadTracker::new();
    // `submit` は完了まで見に行ってしまうので、受付だけを手で組み立てる。
    // （`fold_on_poll` を立てるのは「受付が済んだあと」でなければ隙間を再現できない）
    let request = backend
        .begin_standby(instance_id, None)
        .expect("the fake accepts");
    let request_id = backend.standby_request_id(&request);
    tracker.in_flight = Some(InFlight {
        request,
        instance_id,
        request_id,
        generation: generation.load(Ordering::SeqCst),
        started: Instant::now(),
    });
    tracker
}

#[test]
fn a_completion_observed_after_the_cycle_was_folded_is_stale() {
    let generation: PreloadGeneration = Arc::new(AtomicU64::new(1));
    let mut backend = FoldingBackend::new(&generation);
    let mut tracker = tracker_with_one_in_flight(&generation, &mut backend, 3);

    backend.fold_on_poll = true;
    let outcomes = tracker.advance(&mut backend, &generation);

    assert_eq!(outcomes.len(), 1);
    assert!(
        outcomes[0].stale,
        "世代を「完了を観測したあと」に読んでいれば、畳まれたサイクルの完了は stale"
    );
}

#[test]
fn a_waiting_preload_is_dropped_when_the_cycle_is_folded_while_it_waits() {
    let generation: PreloadGeneration = Arc::new(AtomicU64::new(1));
    let mut backend = FoldingBackend::new(&generation);
    let mut tracker = tracker_with_one_in_flight(&generation, &mut backend, 3);
    // 完了 slot は 1 件ぶんしか無いので、2 件目は受付の順番待ちになる。
    tracker.waiting.push_back(Waiting {
        generation: 1,
        instance_id: 5,
        patch: None,
    });

    backend.fold_on_poll = true;
    let outcomes = tracker.advance(&mut backend, &generation);

    assert_eq!(outcomes.len(), 2);
    assert!(outcomes.iter().all(|outcome| outcome.stale));
    assert_eq!(
        backend.begun,
        vec![3],
        "順番待ちは受付にも出さない（出すとロードしていない bank へ切り替わる）"
    );
}
