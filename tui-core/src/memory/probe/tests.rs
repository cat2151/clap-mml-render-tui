//! 再計測の間引き判定。共有状態を触らずに済むよう、判定だけを直接検証する。

use super::*;

fn idle_at(started_at: Option<Instant>) -> ProbeState {
    ProbeState {
        started_at,
        ..ProbeState::default()
    }
}

/// help を開いた最初の 1 回は無条件で計測する。
#[test]
fn the_first_request_starts_a_measurement() {
    assert!(idle_at(None).should_start(Instant::now()));
}

/// help 表示中は描画のたびに要求が来るので、1 秒未満は間引く。
#[test]
fn requests_within_the_interval_are_skipped() {
    let started_at = Instant::now();
    let state = idle_at(Some(started_at));

    assert!(!state.should_start(started_at + MIN_REFRESH_INTERVAL / 2));
}

/// 1 秒経ったら投げ直し、ハズレ値が表示されたままにならないようにする。
#[test]
fn a_request_after_the_interval_starts_a_measurement() {
    let started_at = Instant::now();
    let state = idle_at(Some(started_at));

    assert!(state.should_start(started_at + MIN_REFRESH_INTERVAL));
}

/// 計測が長引いても多重 spawn はしない。
#[test]
fn an_in_flight_measurement_blocks_a_new_one() {
    let started_at = Instant::now();
    let state = ProbeState {
        in_flight: true,
        started_at: Some(started_at),
        ..ProbeState::default()
    };

    assert!(!state.should_start(started_at + MIN_REFRESH_INTERVAL * 10));
}
