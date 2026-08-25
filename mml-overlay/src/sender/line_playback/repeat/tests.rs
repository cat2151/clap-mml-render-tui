//! 時計を渡して進める。`Instant::now()` を内側で読まないので、待ちも積みも決定的に見られる。

use super::*;

fn event(seconds: f64) -> TimedMidiEvent {
    TimedMidiEvent {
        seconds,
        message: [0x90, 60, 100],
    }
}

fn running(origin: Instant, loop_seconds: f64) -> RepeatState {
    RepeatState::new(
        7,
        origin,
        vec![event(0.0)],
        loop_seconds,
        FilterSettings::default(),
    )
}

fn at(origin: Instant, seconds: f64) -> Instant {
    origin + Duration::from_secs_f64(seconds)
}

/// 張った直後に horizon ぶんだけ先まで積む。0 周目から始まる。
#[test]
fn the_first_fill_covers_the_lookahead_horizon() {
    let origin = Instant::now();
    let mut state = running(origin, 1.0);

    let offsets = state.take_due_cycles(origin);

    assert_eq!(offsets, vec![0.0, 1.0, 2.0, 3.0]);
    assert_eq!(state.take_due_cycles(origin), Vec::<f64>::new());
}

/// 積む周は必ず先へ進む。同じ周を二度返さない。
#[test]
fn cycles_never_repeat_or_go_backwards() {
    let origin = Instant::now();
    let mut state = running(origin, 0.5);
    let mut all = state.take_due_cycles(origin);

    for step in 1..=20 {
        all.extend(state.take_due_cycles(at(origin, step as f64 * 0.5)));
    }

    assert!(all.windows(2).all(|pair| pair[1] > pair[0]));
    assert_eq!(all.first().copied(), Some(0.0));
    // 10 秒経過 + horizon 4 秒 ぶんが埋まっている。
    assert!(*all.last().unwrap() >= 14.0 - 0.5);
}

/// horizon を割るまでは寝ている。割ったら最短の待ちで起こす。
#[test]
fn the_wait_lasts_until_the_horizon_is_broken() {
    let origin = Instant::now();
    let mut state = running(origin, 1.0);
    state.take_due_cycles(origin);

    // 4 秒ぶん積んであるので、次に仕事があるのは 0 秒経過を過ぎた瞬間。
    // それでも 0 は返さない（返すと worker が空回りする）。
    assert_eq!(state.wait(origin), MIN_WAIT);

    let mut state = running(origin, 1.0);
    state.take_due_cycles(at(origin, 2.0));
    // 6 秒まで積んだので、次に仕事があるのは 2 秒経過後。0.5 秒の時点ならあと 1.5 秒。
    assert_eq!(state.wait(at(origin, 0.5)), Duration::from_secs_f64(1.5));
}

/// 時計が大きく飛んでも 1 回で積む量は頭打ちにする。
#[test]
fn a_jumping_clock_does_not_flood_the_server() {
    let origin = Instant::now();
    let mut state = running(origin, MIN_LOOP_SECONDS);

    let offsets = state.take_due_cycles(at(origin, 3_600.0));

    assert_eq!(offsets.len(), MAX_CYCLES_PER_PUMP);
}

/// 1 音だけの行のように 1 周が短すぎるものは繰り返さない。
#[test]
fn a_line_that_is_too_short_is_not_repeatable() {
    assert!(!is_repeatable(0.0));
    assert!(!is_repeatable(MIN_LOOP_SECONDS / 2.0));
    assert!(!is_repeatable(f64::NAN));
    assert!(!is_repeatable(f64::INFINITY));
    assert!(is_repeatable(MIN_LOOP_SECONDS));
    assert!(is_repeatable(2.0));
}

#[test]
fn the_timeline_id_and_the_cycle_are_kept_for_every_lap() {
    let state = running(Instant::now(), 1.0);

    assert_eq!(state.timeline_id(), 7);
    assert_eq!(state.cycle(), &[event(0.0)]);
    assert_eq!(state.loop_seconds(), 1.0);
    // filter の設定は周をまたいで動かない。周ごとに掛け直すのは同じ設定で。
    assert_eq!(state.filters(), FilterSettings::default());
}
