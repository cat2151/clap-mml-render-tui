use super::*;

/// worker の観測周期（`METER_POLL_INTERVAL`）。
const TICK: Duration = Duration::from_millis(50);
/// [`SUSTAINED_WINDOW`] ぶんの観測回数。
const WINDOW_TICKS: u32 = 200;

/// `start` から 50ms 刻みでドロップを観測し続け、成立したら何回目だったかを返す。
fn drop_from(detector: &mut OverloadDetector, start: Instant, ticks: u32) -> Option<u32> {
    (0..=ticks).find(|tick| detector.observe(start + TICK * *tick, true, 4))
}

/// 上限倍率で 1 秒以内の間隔でドロップが続き、それが 10 秒に達したら成立する。
#[test]
fn ten_seconds_of_dropping_at_the_top_of_the_ladder_trips() {
    let mut detector = OverloadDetector::new();

    let tripped_at = drop_from(&mut detector, Instant::now(), WINDOW_TICKS);

    assert_eq!(tripped_at, Some(WINDOW_TICKS), "10秒ちょうどで成立する");
}

/// 10 秒に届かないうちは成立しない。
#[test]
fn a_shorter_streak_does_not_trip() {
    let mut detector = OverloadDetector::new();

    let tripped_at = drop_from(&mut detector, Instant::now(), WINDOW_TICKS - 1);

    assert_eq!(tripped_at, None);
}

/// 途中で 1 秒を超えてドロップが無ければ、連続は切れて数え直しになる。
#[test]
fn a_quiet_second_restarts_the_streak() {
    let now = Instant::now();
    let mut detector = OverloadDetector::new();
    // あと一歩まで連続させてから、静かな観測を1回挟む。
    assert_eq!(drop_from(&mut detector, now, WINDOW_TICKS - 1), None);
    let quiet = now + TICK * WINDOW_TICKS + DROP_GAP;

    assert!(!detector.observe(quiet, true, 0), "静かな観測では立たない");

    // 数え直しなので、ここからさらに 10 秒ぶん続けないと成立しない。
    assert_eq!(drop_from(&mut detector, quiet, WINDOW_TICKS - 1), None);
    assert_eq!(
        drop_from(&mut detector, quiet, WINDOW_TICKS),
        Some(WINDOW_TICKS)
    );
}

/// 梯子の途中でのドロップは数えない。まだバッファを厚くする余地がある。
#[test]
fn dropping_below_the_top_of_the_ladder_is_not_counted() {
    let now = Instant::now();
    let mut detector = OverloadDetector::new();

    for tick in 0..=(WINDOW_TICKS * 2) {
        assert!(
            !detector.observe(now + TICK * tick, false, 4),
            "tick={tick}"
        );
    }
}

/// 一度成立したら降ろさない。降ろすとダブルとシングルを往復してしまう。
#[test]
fn the_verdict_is_latched() {
    let now = Instant::now();
    let mut detector = OverloadDetector::new();
    assert_eq!(
        drop_from(&mut detector, now, WINDOW_TICKS),
        Some(WINDOW_TICKS)
    );
    let after = now + TICK * WINDOW_TICKS;

    // 報せるのは一度だけ。静かになっても、また荒れても立ち上がり直さない。
    assert!(!detector.observe(after, true, 0));
    assert_eq!(drop_from(&mut detector, after, WINDOW_TICKS * 2), None);
    assert!(detector.tripped, "判定そのものは立ったまま");
}
