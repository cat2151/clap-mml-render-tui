use super::*;
use crate::session_state::KeyboardSessionState;

fn arp_state_started_at(anchor: Instant) -> KeyboardState {
    let mut state = KeyboardState::from_session(KeyboardSessionState {
        note_playback_mode: NotePlaybackMode::Arp,
        repeat_chords: vec![vec![60, 64, 67]],
        ..KeyboardSessionState::default()
    });
    assert_eq!(
        state.take_pending_refresh_messages(anchor),
        vec![[0x90, 60, 100]]
    );
    state
}

// 0〜120msの決定的な擬似乱数列(LCG)
fn poll_delays(count: usize) -> Vec<Duration> {
    let mut seed: u32 = 0x1234_5678;
    (0..count)
        .map(|_| {
            seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
            Duration::from_millis(u64::from(seed >> 8) % 121)
        })
        .collect()
}

#[test]
fn tick_deadlines_stay_on_the_anchor_grid_regardless_of_poll_delay() {
    let anchor = Instant::now();
    let mut state = arp_state_started_at(anchor);
    assert_eq!(state.periodic_anchor(), Some(anchor));
    let generation = state.periodic_generation();

    assert_eq!(
        state.poll_periodic_tick(anchor + Duration::from_millis(249)),
        None
    );
    let delays = poll_delays(40);
    assert!(delays.iter().any(|delay| delay.as_millis() >= 100));
    for (index, delay) in delays.into_iter().enumerate() {
        let deadline = at_tick(anchor, index as u64 + 1);
        let polled_at = deadline + delay;
        let tick = state
            .poll_periodic_tick(polled_at)
            .unwrap_or_else(|| panic!("tick {} was skipped", index + 1));
        assert_eq!(tick.at, deadline, "tick {}", index + 1);
        assert_eq!(tick.messages.len(), 2, "arp sends note off + note on");
        // 同じframe内の再pollや次deadline直前のpollでは重複して出ない
        assert_eq!(state.poll_periodic_tick(polled_at), None);
        assert_eq!(
            state.poll_periodic_tick(deadline + Duration::from_millis(249)),
            None
        );
    }
    assert_eq!(state.periodic_anchor(), Some(anchor));
    assert_eq!(state.periodic_generation(), generation);
}

#[test]
fn long_stall_skips_missed_ticks_and_emits_only_one() {
    let anchor = Instant::now();
    let mut state = arp_state_started_at(anchor);
    let resumed_at = anchor + Duration::from_secs(10);

    let tick = state.poll_periodic_tick(resumed_at).expect("one tick");
    assert_eq!(tick.at, at_tick(anchor, 1));
    assert_eq!(state.poll_periodic_tick(resumed_at), None);
    assert_eq!(
        state.poll_periodic_tick(resumed_at + Duration::from_millis(249)),
        None
    );
    let next = state
        .poll_periodic_tick(resumed_at + Duration::from_millis(250))
        .expect("next tick after snapping");
    assert_eq!(next.at, resumed_at + Duration::from_millis(250));
}

#[test]
fn note_playback_cycle_restarts_and_stops_the_clock() {
    let mut state = KeyboardState::default();
    let t0 = Instant::now();
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    assert!(state.release(KEYBOARD_NOTES[0]).is_some());
    assert_eq!(state.periodic_anchor(), None);
    let g0 = state.periodic_generation();

    // off → auto(repeat扱い)で張る
    let t1 = t0 + Duration::from_millis(10);
    assert!(!state.cycle_note_playback(t1).is_empty());
    assert_eq!(state.periodic_anchor(), Some(t1));
    assert!(state.periodic_generation() > g0);
    let g1 = state.periodic_generation();

    // auto(repeat) → repeat は鳴らし続けるので張り直さない
    let t2 = t0 + Duration::from_millis(20);
    assert!(state.cycle_note_playback(t2).is_empty());
    assert_eq!(state.periodic_anchor(), Some(t1));
    assert_eq!(state.periodic_generation(), g1);

    // repeat → arp は張り直す
    let t3 = t0 + Duration::from_millis(30);
    assert!(!state.cycle_note_playback(t3).is_empty());
    assert_eq!(state.periodic_anchor(), Some(t3));
    assert!(state.periodic_generation() > g1);
    let g3 = state.periodic_generation();

    // poll は張り直しではない
    assert!(state.poll_periodic_tick(at_tick(t3, 1)).is_some());
    assert_eq!(state.periodic_generation(), g3);

    // arp → off で止まる
    let t4 = t0 + Duration::from_millis(40);
    state.cycle_note_playback(t4);
    assert_eq!(state.periodic_anchor(), None);
    assert!(state.periodic_generation() > g3);
    assert_eq!(state.poll_periodic_tick(t4 + Duration::from_secs(1)), None);
}

#[test]
fn replacing_chords_restarts_the_clock_at_the_given_time() {
    let anchor = Instant::now();
    let mut state = arp_state_started_at(anchor);
    let generation = state.periodic_generation();
    let replaced_at = anchor + Duration::from_millis(330);

    let messages = state.replace_repeat_chords(vec![vec![62, 65]], replaced_at, true);
    assert_eq!(messages.last(), Some(&[0x90, 62, 100]));
    assert_eq!(state.periodic_anchor(), Some(replaced_at));
    assert!(state.periodic_generation() > generation);
    let tick = state
        .poll_periodic_tick(replaced_at + Duration::from_millis(300))
        .expect("tick");
    assert_eq!(tick.at, at_tick(replaced_at, 1));

    // 次のReady待ちにする差し替えは、まだ張り直さない
    let generation = state.periodic_generation();
    state.replace_repeat_chords(vec![vec![60]], replaced_at + Duration::from_secs(1), false);
    assert_eq!(state.periodic_anchor(), Some(replaced_at));
    assert_eq!(state.periodic_generation(), generation);
}

#[test]
fn periodic_controller_toggles_restart_and_reset_stops_the_clock() {
    let mut state = KeyboardState::default();
    let t0 = Instant::now();
    state.cycle_velocity(t0);
    assert_eq!(state.periodic_anchor(), None);

    let t1 = t0 + Duration::from_millis(70);
    assert_eq!(state.cycle_velocity(t1), VelocityMode::Periodic);
    assert_eq!(state.periodic_anchor(), Some(t1));
    let g1 = state.periodic_generation();

    let t2 = t0 + Duration::from_millis(140);
    state.toggle_cc_periodic(t2);
    assert_eq!(state.periodic_anchor(), Some(t2));
    assert!(state.periodic_generation() > g1);
    let g2 = state.periodic_generation();

    state.take_reset_messages();
    assert_eq!(state.periodic_anchor(), None);
    assert!(state.periodic_generation() > g2);
}
