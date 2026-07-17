use super::*;

#[test]
fn velocity_cycles_normal_accent_periodic_and_applies_to_note_on() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    assert_eq!(state.velocity(), 100);
    assert_eq!(state.cycle_velocity(now), VelocityMode::Accent);
    assert_eq!(state.press(KEYBOARD_NOTES[0]), Some(vec![[0x90, 60, 127]]));
    // 周期突入で即反転(127→100)、1秒ごとにpollが反転する
    assert_eq!(state.cycle_velocity(now), VelocityMode::Periodic);
    assert_eq!(state.velocity(), 100);
    assert!(state
        .poll_periodic(now + Duration::from_millis(500))
        .is_empty());
    assert!(state.poll_periodic(now + Duration::from_secs(1)).is_empty());
    assert_eq!(state.velocity(), 127);
    assert_eq!(state.press(KEYBOARD_NOTES[1]), Some(vec![[0x90, 62, 127]]));
    assert!(state.poll_periodic(now + Duration::from_secs(2)).is_empty());
    assert_eq!(state.velocity(), 100);
    // Periodic→Normalで100固定へ戻り、周期は停止する
    assert_eq!(state.cycle_velocity(now), VelocityMode::Normal);
    assert_eq!(state.velocity(), 100);
    assert!(state.poll_periodic(now + Duration::from_secs(10)).is_empty());
    assert_eq!(state.velocity(), 100);
}

#[test]
fn modulation_cycles_off_on_periodic() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    assert_eq!(state.modulation_mode(), ModulationMode::Off);
    assert_eq!(state.cycle_modulation(now), [0xB0, 1, 127]);
    assert_eq!(state.modulation_mode(), ModulationMode::On);
    // ON(127)の直後なので周期の初回は0を即送信
    assert_eq!(state.cycle_modulation(now), [0xB0, 1, 0]);
    assert_eq!(state.modulation_mode(), ModulationMode::Periodic);
    assert_eq!(state.cycle_modulation(now), [0xB0, 1, 0]);
    assert_eq!(state.modulation_mode(), ModulationMode::Off);
}

#[test]
fn modulation_periodic_alternates_127_and_0_every_second() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    state.cycle_modulation(now);
    assert_eq!(state.cycle_modulation(now), [0xB0, 1, 0]);
    assert!(state
        .poll_periodic(now + Duration::from_millis(999))
        .is_empty());
    assert_eq!(
        state.poll_periodic(now + Duration::from_secs(1)),
        vec![[0xB0, 1, 127]]
    );
    assert_eq!(
        state.poll_periodic(now + Duration::from_secs(2)),
        vec![[0xB0, 1, 0]]
    );
    assert_eq!(
        state.poll_periodic(now + Duration::from_secs(3)),
        vec![[0xB0, 1, 127]]
    );
}

#[test]
fn pitch_bend_cycles_six_steps_with_center_between_extremes() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    assert_eq!(state.pitch_bend_mode(), PitchBendMode::Idle);
    // +8191 → 0 → -8192 → 0 → 周期 → 0 → +8191… の6段サイクル
    assert_eq!(state.cycle_pitch_bend(now), [0xE0, 0x7F, 0x7F]); // +8191
    assert_eq!(state.pitch_bend_mode(), PitchBendMode::Max);
    assert_eq!(state.cycle_pitch_bend(now), [0xE0, 0x00, 0x40]); // 0
    assert_eq!(state.pitch_bend_mode(), PitchBendMode::CenterAfterMax);
    assert_eq!(state.cycle_pitch_bend(now), [0xE0, 0x00, 0x00]); // -8192
    assert_eq!(state.pitch_bend_mode(), PitchBendMode::Min);
    assert_eq!(state.cycle_pitch_bend(now), [0xE0, 0x00, 0x40]); // 0
    assert_eq!(state.pitch_bend_mode(), PitchBendMode::CenterAfterMin);
    assert_eq!(state.cycle_pitch_bend(now), [0xE0, 0x7F, 0x7F]); // 周期開始は+8191
    assert_eq!(state.pitch_bend_mode(), PitchBendMode::Periodic);
    assert_eq!(state.cycle_pitch_bend(now), [0xE0, 0x00, 0x40]); // 周期停止で0
    assert_eq!(state.pitch_bend_mode(), PitchBendMode::CenterAfterCycle);
    assert_eq!(state.cycle_pitch_bend(now), [0xE0, 0x7F, 0x7F]); // 先頭へ戻る
    assert_eq!(state.pitch_bend_mode(), PitchBendMode::Max);
    assert!(state.poll_periodic(now + Duration::from_secs(10)).is_empty());
}

#[test]
fn pitch_bend_periodic_cycles_four_values_every_second() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    for _ in 0..5 {
        state.cycle_pitch_bend(now); // Periodic(+8191送信済み)まで進める
    }
    assert_eq!(state.pitch_bend_mode(), PitchBendMode::Periodic);
    // +8191(即送信済み)→0→-8192→0→+8191…の4値循環
    assert_eq!(
        state.poll_periodic(now + Duration::from_secs(1)),
        vec![[0xE0, 0x00, 0x40]]
    );
    assert_eq!(
        state.poll_periodic(now + Duration::from_secs(2)),
        vec![[0xE0, 0x00, 0x00]]
    );
    assert_eq!(
        state.poll_periodic(now + Duration::from_secs(3)),
        vec![[0xE0, 0x00, 0x40]]
    );
    assert_eq!(
        state.poll_periodic(now + Duration::from_secs(4)),
        vec![[0xE0, 0x7F, 0x7F]]
    );
}

#[test]
fn cc_periodic_toggle_sends_to_configured_cc_number() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    state.begin_numeric_input(NumericInputTarget::CcNumber);
    state.numeric_input_push('7');
    state.numeric_input_push('4');
    assert_eq!(state.confirm_numeric_input(), None);

    // OFF状態は実質0なので周期の初回は127を即送信
    assert_eq!(state.toggle_cc_periodic(now), [0xB0, 74, 127]);
    assert!(state.cc_periodic_on());
    assert_eq!(
        state.poll_periodic(now + Duration::from_secs(1)),
        vec![[0xB0, 74, 0]]
    );
    assert_eq!(
        state.poll_periodic(now + Duration::from_secs(2)),
        vec![[0xB0, 74, 127]]
    );
    assert_eq!(state.toggle_cc_periodic(now), [0xB0, 74, 0]);
    assert!(!state.cc_periodic_on());
    assert!(state.poll_periodic(now + Duration::from_secs(10)).is_empty());
}

#[test]
fn note_repeat_retriggers_last_chord_every_second() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    // c e g を同時押し→全release。和音は保持される
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    assert!(state.press(KEYBOARD_NOTES[2]).is_some());
    assert!(state.press(KEYBOARD_NOTES[4]).is_some());
    assert!(state.release(KEYBOARD_NOTES[0]).is_some());
    assert!(state.release(KEYBOARD_NOTES[2]).is_some());
    assert!(state.release(KEYBOARD_NOTES[4]).is_some());

    // ON: 即座に和音を発音
    assert_eq!(
        state.toggle_note_repeat(now),
        vec![[0x90, 60, 100], [0x90, 64, 100], [0x90, 67, 100]]
    );
    assert!(state.note_repeat_on());
    // 毎秒リトリガー: off+onを同時送信
    assert_eq!(
        state.poll_periodic(now + Duration::from_secs(1)),
        vec![
            [0x80, 60, 0],
            [0x80, 64, 0],
            [0x80, 67, 0],
            [0x90, 60, 100],
            [0x90, 64, 100],
            [0x90, 67, 100],
        ]
    );
    // OFF: 発音中ノートを止める
    assert_eq!(
        state.toggle_note_repeat(now),
        vec![[0x80, 60, 0], [0x80, 64, 0], [0x80, 67, 0]]
    );
    assert!(!state.note_repeat_on());
    assert!(state.poll_periodic(now + Duration::from_secs(10)).is_empty());
}

#[test]
fn note_repeat_does_nothing_without_chord() {
    let mut state = KeyboardState::default();
    assert!(state.toggle_note_repeat(Instant::now()).is_empty());
    assert!(!state.note_repeat_on());
}

#[test]
fn note_repeat_uses_current_velocity_on_each_retrigger() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    assert!(state.release(KEYBOARD_NOTES[0]).is_some());
    state.cycle_velocity(now); // Accent(127)
    assert_eq!(state.toggle_note_repeat(now), vec![[0x90, 60, 127]]);
    assert_eq!(
        state.poll_periodic(now + Duration::from_secs(1)),
        vec![[0x80, 60, 0], [0x90, 60, 127]]
    );
}

#[test]
fn poll_periodic_is_empty_without_periodic_modes() {
    let mut state = KeyboardState::default();
    assert!(state
        .poll_periodic(Instant::now() + Duration::from_secs(60))
        .is_empty());
}

#[test]
fn poll_periodic_combines_channels_in_fixed_order() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    // 全チャネルを周期modeへ(mod→PB→CCの順で送出される)
    state.cycle_modulation(now);
    state.cycle_modulation(now);
    for _ in 0..5 {
        state.cycle_pitch_bend(now);
    }
    state.toggle_cc_periodic(now);

    assert_eq!(
        state.poll_periodic(now + Duration::from_secs(1)),
        vec![[0xB0, 1, 127], [0xE0, 0x00, 0x40], [0xB0, 1, 0]]
    );
}

#[test]
fn poll_periodic_skips_missed_cycles_after_long_stall() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    state.cycle_modulation(now);
    state.cycle_modulation(now);

    // 大幅遅延時は1発のみ送出し、次回はnow基準の1秒後へスナップ
    assert_eq!(
        state.poll_periodic(now + Duration::from_secs(10)),
        vec![[0xB0, 1, 127]]
    );
    assert!(state
        .poll_periodic(now + Duration::from_millis(10_500))
        .is_empty());
    assert_eq!(
        state.poll_periodic(now + Duration::from_secs(11)),
        vec![[0xB0, 1, 0]]
    );
}

#[test]
fn take_note_off_messages_keeps_auto_send_modes_and_schedules_refresh() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    state.begin_numeric_input(NumericInputTarget::CcNumber);
    state.numeric_input_push('7');
    state.numeric_input_push('4');
    assert_eq!(state.confirm_numeric_input(), None);

    state.cycle_modulation(now); // On
    state.cycle_pitch_bend(now); // Max
    state.toggle_cc_periodic(now); // 127送信済み(次は0)
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    let _ = state.toggle_note_repeat(now);

    // patch変更: note offのみ送出し、modeは維持される
    assert_eq!(
        state.take_note_off_messages(),
        vec![[0x80, 60, 0], [0x80, 60, 0]]
    );
    assert_eq!(state.modulation_mode(), ModulationMode::On);
    assert_eq!(state.pitch_bend_mode(), PitchBendMode::Max);
    assert!(state.cc_periodic_on());
    assert!(state.note_repeat_on());

    // Ready復帰時に現在値+リトリガー和音を再送
    assert_eq!(
        state.take_pending_refresh_messages(),
        vec![
            [0xB0, 1, 127],     // modulation ON
            [0xE0, 0x7F, 0x7F], // pitch bend +8191
            [0xB0, 74, 127],    // 汎用CC現在値
            [0x90, 60, 100],    // note repeat和音の再発音
        ]
    );
    assert!(state.take_pending_refresh_messages().is_empty());
}

#[test]
fn refresh_resends_current_periodic_values() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    state.cycle_modulation(now);
    state.cycle_modulation(now); // Periodic(0送信済み)
    assert_eq!(
        state.poll_periodic(now + Duration::from_secs(1)),
        vec![[0xB0, 1, 127]]
    );

    let _ = state.take_note_off_messages();
    // 周期中の現在値(127)を再送する
    assert_eq!(
        state.take_pending_refresh_messages(),
        vec![[0xB0, 1, 127]]
    );
}

#[test]
fn take_reset_messages_clears_pending_refresh() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    state.cycle_modulation(now); // On
    let _ = state.take_note_off_messages();
    let _ = state.take_reset_messages();
    assert!(state.take_pending_refresh_messages().is_empty());
}
