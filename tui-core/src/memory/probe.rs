//! 計測結果を保持するプロセスグローバルな共有状態と、計測スレッドの起動。

use std::sync::{Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use super::{MemoryReading, MemorySnapshot};

/// 再計測を投げ直す最短間隔。
///
/// 計測値はその瞬間の Working Set なので一時的にハズレ値が出る。help 表示中は
/// 描画のたび（50ms 間隔）に再計測要求が来るため、ここまで間引いたうえで
/// 値を更新し続け、ハズレ値が出たまま固まってユーザーが誤読するのを防ぐ。
const MIN_REFRESH_INTERVAL: Duration = Duration::from_secs(1);

#[derive(Default)]
struct ProbeState {
    /// 直近の計測結果。再計測中も保持して、help を開き直したときに前回値を即出す。
    last: Option<MemorySnapshot>,
    /// 計測スレッドが動いている間だけ true。多重 spawn を防ぐ。
    in_flight: bool,
    /// 一度も計測に成功していない状態で失敗したかどうか。
    failed: bool,
    /// 直近に計測スレッドを起動した時刻。間引き判定に使う。
    started_at: Option<Instant>,
}

impl ProbeState {
    fn should_start(&self, now: Instant) -> bool {
        if self.in_flight {
            return false;
        }
        match self.started_at {
            Some(started_at) => now.duration_since(started_at) >= MIN_REFRESH_INTERVAL,
            None => true,
        }
    }
}

fn state() -> &'static Mutex<ProbeState> {
    static STATE: OnceLock<Mutex<ProbeState>> = OnceLock::new();
    STATE.get_or_init(|| Mutex::new(ProbeState::default()))
}

/// 計測結果は次回の計測で上書きされる使い捨ての値なので、poison しても続行する。
fn lock() -> MutexGuard<'static, ProbeState> {
    state()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
}

pub(super) fn request_refresh(measure: fn() -> Option<MemorySnapshot>) {
    // テストでは実測しない。マシンごとに値が変わると、値の桁数に依存しない作りに
    // していても計測タイミング次第で表示状態が変わり、描画テストが不安定になるため。
    // `test-support` 有効時は常に「計測中」表示のままになる。
    if cfg!(feature = "test-support") {
        return;
    }

    {
        let now = Instant::now();
        let mut state = lock();
        if !state.should_start(now) {
            return;
        }
        state.in_flight = true;
        state.started_at = Some(now);
    }

    let spawned = std::thread::Builder::new()
        .name("cmrt-memory-probe".to_string())
        .spawn(move || {
            let measured = measure();
            let mut state = lock();
            state.failed = measured.is_none();
            if measured.is_some() {
                state.last = measured;
            }
            state.in_flight = false;
        });

    if spawned.is_err() {
        let mut state = lock();
        state.in_flight = false;
        state.failed = true;
    }
}

#[cfg(test)]
mod tests;

pub(super) fn reading() -> MemoryReading {
    let state = lock();
    match &state.last {
        Some(snapshot) => MemoryReading::Ready(snapshot.clone()),
        None if state.failed => MemoryReading::Unavailable,
        None => MemoryReading::Measuring,
    }
}
