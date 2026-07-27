//! 計測結果を保持するプロセスグローバルな共有状態と、計測スレッドの起動。

use std::sync::{Mutex, MutexGuard, OnceLock};

use super::{MemoryReading, MemorySnapshot};

#[derive(Default)]
struct ProbeState {
    /// 直近の計測結果。再計測中も保持して、help を開き直したときに前回値を即出す。
    last: Option<MemorySnapshot>,
    /// 計測スレッドが動いている間だけ true。多重 spawn を防ぐ。
    in_flight: bool,
    /// 一度も計測に成功していない状態で失敗したかどうか。
    failed: bool,
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
        let mut state = lock();
        if state.in_flight {
            return;
        }
        state.in_flight = true;
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

pub(super) fn reading() -> MemoryReading {
    let state = lock();
    match state.last {
        Some(snapshot) => MemoryReading::Ready(snapshot),
        None if state.failed => MemoryReading::Unavailable,
        None => MemoryReading::Measuring,
    }
}
