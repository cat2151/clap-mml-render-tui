use super::*;

const EPS: f64 = 1e-9;
const HORIZON: f64 = 0.25;

fn hit(seconds: f64, note: u8) -> StepHit {
    StepHit {
        seconds,
        note,
        velocity: 127,
        gate_seconds: 2.0,
    }
}

fn state(origin: Instant, hits: Vec<StepHit>) -> StepLoopState {
    StepLoopState::new(
        origin,
        StepLoop {
            loop_seconds: 2.0,
            hits,
            horizon_seconds: HORIZON,
        },
    )
    .unwrap()
}

fn at(origin: Instant, seconds: f64) -> Instant {
    origin + Duration::from_secs_f64(seconds)
}

/// `until` 秒まで 0.1 秒ごとに起きたとして、積んだものを全部返す。
fn pump_until(
    state: &mut StepLoopState,
    origin: Instant,
    from: f64,
    until: f64,
) -> Vec<(f64, [u8; 3])> {
    let mut events = Vec::new();
    let mut seconds = from;
    while seconds <= until + EPS {
        events.extend(
            state
                .take_due_events(at(origin, seconds))
                .into_iter()
                .map(|event| (event.seconds, event.message)),
        );
        seconds += 0.1;
    }
    events
}

fn ons(events: &[(f64, [u8; 3])]) -> Vec<(f64, u8)> {
    events
        .iter()
        .filter(|(_, message)| message[0] == NOTE_ON)
        .map(|(seconds, message)| (*seconds, message[1]))
        .collect()
}

fn assert_close(actual: &[(f64, u8)], expected: &[(f64, u8)]) {
    assert_eq!(actual.len(), expected.len(), "{actual:?} != {expected:?}");
    for (left, right) in actual.iter().zip(expected) {
        assert!(
            (left.0 - right.0).abs() < 1e-6 && left.1 == right.1,
            "{actual:?} != {expected:?}"
        );
    }
}

#[test]
fn the_cycle_stays_two_seconds_and_a_same_note_off_precedes_its_next_on() {
    let origin = Instant::now();
    let mut state = state(origin, vec![hit(1.875, 36), hit(0.0, 36), hit(0.0, 38)]);
    let events = pump_until(&mut state, origin, 0.0, 4.1);
    assert_close(
        &ons(&events),
        &[
            (0.0, 36),
            (0.0, 38),
            (1.875, 36),
            (2.0, 36),
            (2.0, 38),
            (3.875, 36),
            (4.0, 36),
            (4.0, 38),
        ],
    );
    // 時刻は前へ戻らず、同じ時刻なら off が先。
    assert!(events.windows(2).all(|pair| pair[0].0 <= pair[1].0 + EPS));
    for (index, (seconds, message)) in events.iter().enumerate() {
        if message[0] == NOTE_ON && *seconds > 0.0 {
            let off = events[..index]
                .iter()
                .rposition(|(_, earlier)| earlier[0] == NOTE_OFF && earlier[1] == message[1]);
            let off = off.expect("each re-hit is preceded by an off of the same note");
            assert!(
                (events[off].0 - seconds).abs() < 1e-6,
                "the previous note is cut exactly at the re-hit: {events:?}"
            );
        }
    }
    // 1.875 の 36 は 0.125 秒で切れ、2 秒 gate のまま残って 2.0 の 36 を消すことはない。
    let offs_36: Vec<f64> = events
        .iter()
        .filter(|(_, message)| message[0] == NOTE_OFF && message[1] == 36)
        .map(|(seconds, _)| *seconds)
        .collect();
    assert_eq!(offs_36.len(), 4, "{offs_36:?}");
    assert!((offs_36[0] - 1.875).abs() < 1e-6 && (offs_36[1] - 2.0).abs() < 1e-6);
}

#[test]
fn editing_reaches_only_unscheduled_times_without_returning_to_the_start() {
    let origin = Instant::now();
    let mut state = state(origin, vec![hit(0.0, 36)]);
    let before = pump_until(&mut state, origin, 0.0, 0.5);
    // 0.5 秒時点では 0.75 秒まで積んである。0.5 秒の打点はもう間に合わない。
    state.set_hits(vec![hit(0.0, 36), hit(0.5, 38), hit(1.0, 42)]);
    let after = pump_until(&mut state, origin, 0.6, 2.7);
    assert_close(&ons(&before), &[(0.0, 36)]);
    assert_close(&ons(&after), &[(1.0, 42), (2.0, 36), (2.5, 38)]);
    assert!(
        after.iter().all(|(seconds, _)| *seconds >= 0.75 - EPS),
        "nothing is re-sent from the head: {after:?}"
    );
}

#[test]
fn an_empty_loop_keeps_its_clock_and_plays_hits_added_later() {
    let origin = Instant::now();
    let mut state = state(origin, Vec::new());
    assert!(pump_until(&mut state, origin, 0.0, 1.0).is_empty());
    state.set_hits(vec![hit(0.25, 40)]);
    let events = pump_until(&mut state, origin, 1.1, 2.4);
    assert_close(&ons(&events), &[(2.25, 40)]);
    // velocity 0 と周の外の打点は鳴らさない。
    state.set_hits(vec![
        StepHit {
            velocity: 0,
            ..hit(0.0, 41)
        },
        hit(2.0, 43),
        hit(-0.1, 44),
    ]);
    assert!(ons(&pump_until(&mut state, origin, 2.5, 4.5)).is_empty());
}

#[test]
fn a_late_wake_skips_past_hits_but_still_releases_notes() {
    let origin = Instant::now();
    let mut state = state(origin, vec![hit(0.0, 36), hit(0.5, 38)]);
    state.take_due_events(at(origin, 0.0));
    let late = state.take_due_events(at(origin, 1.9));
    assert!(ons(&late
        .iter()
        .map(|event| (event.seconds, event.message))
        .collect::<Vec<_>>())
    .iter()
    .all(|(seconds, _)| *seconds >= 1.9 - EPS));
    // 36 は 0.0 で鳴らした。その off（2.0）は次の 36 の on と同時に出る。
    assert!(late
        .iter()
        .any(|event| event.message == [NOTE_OFF, 36, 0] && (event.seconds - 2.0).abs() < 1e-6));
}

#[test]
fn the_worker_wakes_while_half_of_the_lookahead_remains() {
    let origin = Instant::now();
    let mut state = state(origin, Vec::new());
    state.take_due_events(at(origin, 1.0));
    let wait = state.wait(at(origin, 1.0)).as_secs_f64();
    assert!((wait - HORIZON / 2.0).abs() < 1e-3, "{wait}");
    assert_eq!(state.wait(at(origin, 5.0)), MIN_WAIT);
    assert!(StepLoopState::new(
        origin,
        StepLoop {
            loop_seconds: 0.0,
            hits: Vec::new(),
            horizon_seconds: HORIZON,
        },
    )
    .is_none());
}

#[test]
fn each_hit_releases_after_its_own_gate_even_past_the_cycle_end() {
    let origin = Instant::now();
    let short = StepHit {
        gate_seconds: 0.125,
        ..hit(0.0, 36)
    };
    let long = StepHit {
        gate_seconds: 0.5,
        ..hit(1.75, 49)
    };
    let mut state = state(origin, vec![short, long]);
    let offs: Vec<(f64, u8)> = pump_until(&mut state, origin, 0.0, 2.6)
        .into_iter()
        .filter(|(_, message)| message[0] == NOTE_OFF)
        .map(|(seconds, message)| (seconds, message[1]))
        .collect();
    assert_close(&offs, &[(0.125, 36), (2.125, 36), (2.25, 49)]);
}

fn shot(note: u8, gate_seconds: f64) -> StepShot {
    StepShot {
        note,
        velocity: 127,
        gate_seconds,
    }
}

fn messages(events: Vec<TimedMidiEvent>) -> Vec<(f64, [u8; 3])> {
    events
        .into_iter()
        .map(|event| (event.seconds, event.message))
        .collect()
}

#[test]
fn a_shot_sounds_now_ahead_of_the_scheduled_window() {
    let origin = Instant::now();
    let mut state = state(origin, Vec::new());
    state.take_due_events(at(origin, 0.5));
    let events = messages(state.take_shot(at(origin, 0.5), shot(38, 0.125)));
    // 0.75 まで積んであっても、単発はいまの 0.5 に置き、積み済みの 0.625 の off も出す。
    assert_eq!(events.len(), 2, "{events:?}");
    assert!((events[0].0 - 0.5).abs() < 1e-6 && events[0].1 == [NOTE_ON, 38, 127]);
    assert!((events[1].0 - 0.625).abs() < 1e-6 && events[1].1 == [NOTE_OFF, 38, 0]);
    // off はもう出したので、次の積み込みで二重に出ない。
    let later = pump_until(&mut state, origin, 0.6, 1.0);
    assert!(
        later.iter().all(|(_, message)| message[1] != 38),
        "{later:?}"
    );
}

#[test]
fn a_shot_cuts_a_sounding_note_and_is_cut_by_the_next_hit() {
    let origin = Instant::now();
    let mut state = state(origin, vec![hit(0.0, 36)]);
    pump_until(&mut state, origin, 0.0, 0.5);
    let events = messages(state.take_shot(at(origin, 0.5), shot(36, 4.0)));
    assert_eq!(
        events
            .iter()
            .map(|(_, message)| *message)
            .collect::<Vec<_>>(),
        vec![[NOTE_OFF, 36, 0], [NOTE_ON, 36, 127]],
        "{events:?}"
    );
    let next = pump_until(&mut state, origin, 0.6, 2.0);
    // 4 秒 gate の単発は、2.0 の 36 で切れる。
    assert!(next
        .iter()
        .any(|(seconds, message)| *message == [NOTE_OFF, 36, 0] && (seconds - 2.0).abs() < 1e-6));
}

#[test]
fn a_shot_waits_for_a_scheduled_off_and_yields_to_a_scheduled_on() {
    let origin = Instant::now();
    let short = StepHit {
        gate_seconds: 0.125,
        ..hit(0.5, 36)
    };
    let mut state = state(origin, vec![short]);
    // 0.45 秒時点で 0.7 まで積み、0.5 の on と 0.625 の off は積み済み。
    state.take_due_events(at(origin, 0.45));
    assert!(
        state
            .take_shot(at(origin, 0.45), shot(36, 0.125))
            .is_empty(),
        "the scheduled 0.5 hit plays instead"
    );
    let events = messages(state.take_shot(at(origin, 0.55), shot(36, 0.125)));
    // 積み済みの 0.625 の off に切られないよう、そこまで遅らせる。
    assert!(
        (events[0].0 - 0.625).abs() < 1e-6 && events[0].1 == [NOTE_ON, 36, 127],
        "{events:?}"
    );
}

#[test]
fn a_new_horizon_changes_how_far_ahead_and_how_often() {
    let origin = Instant::now();
    let mut state = state(origin, Vec::new());
    state.set_horizon(0.05);
    state.take_due_events(at(origin, 1.0));
    let wait = state.wait(at(origin, 1.0)).as_secs_f64();
    assert!((wait - 0.025).abs() < 1e-3, "{wait}");
    // 回せない値は無視する。
    state.set_horizon(0.0);
    state.set_horizon(f64::NAN);
    let wait = state.wait(at(origin, 1.0)).as_secs_f64();
    assert!((wait - 0.025).abs() < 1e-3, "{wait}");
    // 短くした先読みでは、差し替えがそのぶん早く届く。
    state.set_hits(vec![hit(1.1, 40)]);
    let events = pump_until(&mut state, origin, 1.1, 1.2);
    assert_close(&ons(&events), &[(1.1, 40)]);
}
