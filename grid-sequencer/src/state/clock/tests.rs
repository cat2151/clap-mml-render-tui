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
    assert_eq!(clock.take_due(now, Duration::ZERO)[0].deadline, now);
}

#[test]
fn lookahead_returns_every_step_inside_the_window() {
    let now = Instant::now();
    let mut clock = StepClock::default();
    clock.start(now);

    // 2ステップぶんの先読みなので、締切 0 / 1 / 2 ステップ目までが入る。
    let due = clock.take_due(now, LOOKAHEAD);

    assert_eq!(due.len(), 3);
    assert_eq!(due[0].deadline, now);
    assert_eq!(due[1].deadline, now + STEP_INTERVAL);
    assert_eq!(due[2].deadline, now + STEP_INTERVAL * 2);
    assert_eq!(
        due.iter().map(|due| due.step).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
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
    assert_eq!(due[16].deadline, now + Duration::from_nanos(1_846_153_846));
    // STEP_INTERVAL を16回足すと 6ns 手前になる。絶対位置で計算する理由がこれ。
    assert!(due[16].deadline > now + STEP_INTERVAL * 16);
}

#[test]
fn manual_decimal_bpm_uses_absolute_deadlines_without_accumulation() {
    let bpm = 123.456789;
    let step = 100_000;
    let direct = step_offset_at(step, bpm);
    let repeated = step_interval_at(bpm) * u32::try_from(step).unwrap();
    let mathematical = Duration::from_nanos((step as f64 * 60_000_000_000.0 / (bpm * 4.0)) as u64);

    assert_eq!(direct, mathematical);
    assert_ne!(direct, repeated);
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
        clock.take_due(late, STEP_INTERVAL)[0].deadline,
        now + STEP_INTERVAL
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

    assert!(
        due.is_empty(),
        "the next absolute step is slightly after resumed"
    );
    let resumed_step = clock.take_due(resumed + STEP_INTERVAL, Duration::ZERO);
    assert_eq!(resumed_step.len(), 1);
    assert!(resumed_step[0].step > 1);
}

#[test]
fn retempo_keeps_the_boundary_step_in_place_and_only_widens_what_follows() {
    let now = Instant::now();
    let mut clock = StepClock::default();
    clock.start(now);
    // 16ステップぶん取り出す（0..=16）。17番目が次の周の頭。
    let due = clock.take_due(now, step_offset(16));
    let wrap = due[16];
    assert_eq!(wrap.step, 16);

    let wrap_seconds = clock.timeline_seconds(wrap.step);
    clock.retempo(wrap.step, 65.0);

    // 周の頭そのものは、変更前のテンポで決まった位置のまま動かさない。
    assert_eq!(clock.timeline_seconds(wrap.step), wrap_seconds);
    // 続くステップだけが新しい間隔になる。BPM65 は BPM130 のちょうど倍の長さ。
    let next = clock.take_due(wrap.deadline + step_offset_at(1, 65.0), Duration::ZERO);
    assert_eq!(next.len(), 1);
    assert_eq!(next[0].step, 17);
    assert_eq!(next[0].deadline, wrap.deadline + STEP_INTERVAL * 2);
    assert!(clock.timeline_seconds(17) > wrap_seconds);
}

#[test]
fn retempo_never_rewinds_the_musical_timeline() {
    let now = Instant::now();
    let mut clock = StepClock::default();
    clock.start(now);
    let mut last = f64::NEG_INFINITY;
    let mut at = now;

    // テンポを上げ下げしながら 8 周ぶん進めても、絶対 musical time は単調増加する。
    for (cycle, bpm) in [160.0, 80.0, 130.0, 200.0, 90.0, 140.0, 100.0, 120.0]
        .into_iter()
        .enumerate()
    {
        at += Duration::from_secs(5);
        for due in clock.take_due(at, Duration::ZERO) {
            let seconds = clock.timeline_seconds(due.step);
            assert!(seconds >= last, "cycle={cycle} {last} -> {seconds}");
            last = seconds;
        }
        let step = clock.next_step;
        clock.retempo(step, bpm);
        assert!(clock.timeline_seconds(step) >= last);
    }
}

#[test]
fn retempo_on_a_stopped_clock_does_nothing() {
    let mut clock = StepClock::default();
    clock.retempo(0, 90.0);
    assert!(!clock.is_running());
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
