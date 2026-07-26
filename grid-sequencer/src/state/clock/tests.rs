use super::*;

#[test]
fn the_step_length_is_a_bpm130_sixteenth_note() {
    // 60/130/4 = 115.3846ms。from_millis(115) だと16ステップで6msずれる。
    assert_eq!(STEP_INTERVAL, Duration::from_nanos(115_384_615));
    assert_ne!(STEP_INTERVAL, Duration::from_millis(115));
}

#[test]
fn stopped_clock_never_fires() {
    let mut clock = StepClock::default();
    assert!(!clock.is_running());
    assert!(clock
        .take_due(Instant::now() + Duration::from_secs(60), LOOKAHEAD)
        .is_empty());
}

#[test]
fn start_fires_immediately_so_the_first_step_sounds_on_entry() {
    let now = Instant::now();
    let mut clock = StepClock::default();
    clock.start(now);

    assert!(clock.is_running());
    assert_eq!(clock.take_due(now, Duration::ZERO), vec![now]);
}

#[test]
fn lookahead_returns_every_step_inside_the_window() {
    let now = Instant::now();
    let mut clock = StepClock::default();
    clock.start(now);

    // 2ステップぶんの先読みなので、締切 0 / 1 / 2 ステップ目までが入る。
    let due = clock.take_due(now, LOOKAHEAD);

    assert_eq!(due.len(), 3);
    assert_eq!(due[0], now);
    assert_eq!(due[1], now + STEP_INTERVAL);
    assert_eq!(due[2], now + STEP_INTERVAL * 2);
    // 同じ now で呼び直しても二重に返さない。
    assert!(clock.take_due(now, LOOKAHEAD).is_empty());
}

#[test]
fn deadlines_do_not_drift_over_a_full_bar() {
    let now = Instant::now();
    let mut clock = StepClock::default();
    clock.start(now);

    // 16ステップぶん一気に取り出し、最後の締切が絶対位置と一致することを見る。
    let due = clock.take_due(now, step_offset(16));

    assert_eq!(due.len(), 17);
    // 16ステップ = 60秒 * 16 / (130*4) = 1.846153846 秒。
    assert_eq!(due[16], now + Duration::from_nanos(1_846_153_846));
    // STEP_INTERVAL を16回足すと 6ns 手前になる。絶対位置で計算する理由がこれ。
    assert!(due[16] > now + STEP_INTERVAL * 16);
}

#[test]
fn polling_slightly_late_keeps_the_original_phase() {
    let now = Instant::now();
    let mut clock = StepClock::default();
    clock.start(now);
    assert_eq!(clock.take_due(now, Duration::ZERO).len(), 1);

    // 10ms 遅れて呼んでも、次の締切は 1ステップ後のまま。
    let late = now + Duration::from_millis(10);
    assert_eq!(
        clock.take_due(late, STEP_INTERVAL),
        vec![now + STEP_INTERVAL]
    );
}

#[test]
fn large_delay_snaps_to_now_instead_of_bursting_the_missed_steps() {
    let now = Instant::now();
    let mut clock = StepClock::default();
    clock.start(now);
    assert_eq!(clock.take_due(now, Duration::ZERO).len(), 1);

    // 10秒停滞したあとの復帰。欠落した約87ステップをまとめて発火させない。
    let resumed = now + Duration::from_secs(10);
    let due = clock.take_due(resumed, Duration::ZERO);

    assert_eq!(due, vec![resumed]);
    assert!(clock.take_due(resumed, Duration::ZERO).is_empty());
    assert_eq!(
        clock.take_due(resumed + STEP_INTERVAL, Duration::ZERO),
        vec![resumed + STEP_INTERVAL]
    );
}

#[test]
fn stop_halts_the_progression() {
    let now = Instant::now();
    let mut clock = StepClock::default();
    clock.start(now);
    clock.stop();

    assert!(!clock.is_running());
    assert!(clock
        .take_due(now + Duration::from_secs(1), LOOKAHEAD)
        .is_empty());
}

#[test]
fn frames_ahead_converts_a_step_at_48khz() {
    // 48000 * 60 / (130*4) = 5538.46 サンプル。切り捨てで 5538。
    assert_eq!(frames_ahead(STEP_INTERVAL, 48_000.0), 5538);
    assert_eq!(frames_ahead(Duration::ZERO, 48_000.0), 0);
    assert_eq!(frames_ahead(Duration::from_secs(1), 44_100.0), 44_100);
}

#[test]
fn frames_ahead_returns_zero_for_a_nonsense_sample_rate() {
    assert_eq!(frames_ahead(STEP_INTERVAL, 0.0), 0);
    assert_eq!(frames_ahead(STEP_INTERVAL, -1.0), 0);
}

#[test]
fn the_lookahead_beats_the_ui_polling_interval() {
    // UI ポーリング（数十ms）と出力リングのレイテンシを下回ると、offset が過去になり
    // サーバー側で 0 へクランプされて先読みの意味がなくなる。
    assert!(LOOKAHEAD >= Duration::from_millis(200));
    assert!(SCHEDULE_GUARD < STEP_INTERVAL);
}
