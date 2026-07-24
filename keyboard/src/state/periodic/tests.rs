use std::collections::HashSet;

use super::*;

mod repeat;

fn set_cc_number_74(state: &mut KeyboardState) {
    state.begin_numeric_input(NumericInputTarget::CcNumber);
    state.numeric_input_push('7');
    state.numeric_input_push('4');
    assert_eq!(state.confirm_numeric_input(), None);
}

fn at_tick(now: Instant, tick: u64) -> Instant {
    now + Duration::from_millis(250 * tick)
}

#[test]
fn velocity_cycles_normal_accent_periodic_and_applies_to_note_on() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    assert_eq!(state.velocity(), 100);
    assert_eq!(state.cycle_velocity(now), VelocityMode::Accent);
    assert_eq!(state.press(KEYBOARD_NOTES[0]), Some(vec![[0x90, 60, 127]]));
    // 周期突入で即反転(127→100)、以降は毎tickのbag値
    assert_eq!(state.cycle_velocity(now), VelocityMode::Periodic);
    assert_eq!(state.velocity(), 100);
    assert!(state
        .poll_periodic(now + Duration::from_millis(249))
        .is_empty());
    // 2-bag: 2tickで100と127の両方が1回ずつ現れる
    let mut seen = HashSet::new();
    for tick in 1..=2 {
        assert!(state.poll_periodic(at_tick(now, tick)).is_empty());
        seen.insert(state.velocity());
    }
    assert_eq!(seen, HashSet::from([100, 127]));
    // Periodic→Normalで100固定へ戻り、周期は停止する
    assert_eq!(state.cycle_velocity(now), VelocityMode::Normal);
    assert_eq!(state.velocity(), 100);
    assert!(state
        .poll_periodic(now + Duration::from_secs(10))
        .is_empty());
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
fn modulation_periodic_covers_both_values_in_each_bag() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    state.cycle_modulation(now);
    assert_eq!(state.cycle_modulation(now), [0xB0, 1, 0]);
    assert!(state
        .poll_periodic(now + Duration::from_millis(249))
        .is_empty());
    // 2-bag: 2tickごとに0と127が1回ずつ現れる
    for bag in 0..2u64 {
        let mut values = Vec::new();
        for tick in 1..=2 {
            let messages = state.poll_periodic(at_tick(now, bag * 2 + tick));
            assert_eq!(messages.len(), 1);
            assert_eq!(&messages[0][..2], &[0xB0, 1]);
            values.push(messages[0][2]);
        }
        values.sort_unstable();
        assert_eq!(values, vec![0, 127]);
    }
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
    assert!(state
        .poll_periodic(now + Duration::from_secs(10))
        .is_empty());
}

#[test]
fn pitch_bend_periodic_covers_cycle_values_in_each_bag() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    for _ in 0..5 {
        state.cycle_pitch_bend(now); // Periodic(+8191送信済み)まで進める
    }
    assert_eq!(state.pitch_bend_mode(), PitchBendMode::Periodic);
    // 4-bag: 4tickでPITCH_BEND_CYCLEの4値(0は2回)が1回ずつ現れる
    let mut values = Vec::new();
    for tick in 1..=4 {
        let messages = state.poll_periodic(at_tick(now, tick));
        assert_eq!(messages.len(), 1);
        values.push(messages[0]);
    }
    values.sort_unstable();
    let mut expected = vec![
        [0xE0, 0x7F, 0x7F],
        [0xE0, 0x00, 0x40],
        [0xE0, 0x00, 0x00],
        [0xE0, 0x00, 0x40],
    ];
    expected.sort_unstable();
    assert_eq!(values, expected);
}

#[test]
fn cc_periodic_toggle_sends_to_configured_cc_number() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    set_cc_number_74(&mut state);

    // OFF状態は実質0なので周期の初回は127を即送信
    assert_eq!(state.toggle_cc_periodic(now), [0xB0, 74, 127]);
    assert!(state.cc_periodic_on());
    // 2-bag: 2tickで0と127が1回ずつ現れる
    let mut values = Vec::new();
    for tick in 1..=2 {
        let messages = state.poll_periodic(at_tick(now, tick));
        assert_eq!(messages.len(), 1);
        assert_eq!(&messages[0][..2], &[0xB0, 74]);
        values.push(messages[0][2]);
    }
    values.sort_unstable();
    assert_eq!(values, vec![0, 127]);
    assert_eq!(state.toggle_cc_periodic(now), [0xB0, 74, 0]);
    assert!(!state.cc_periodic_on());
    assert!(state
        .poll_periodic(now + Duration::from_secs(10))
        .is_empty());
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
    set_cc_number_74(&mut state);
    // 全チャネルを周期modeへ(mod→PB→CCの順で送出される)
    state.cycle_modulation(now);
    state.cycle_modulation(now);
    for _ in 0..5 {
        state.cycle_pitch_bend(now);
    }
    state.toggle_cc_periodic(now);

    let messages = state.poll_periodic(at_tick(now, 1));
    assert_eq!(messages.len(), 3);
    assert_eq!(&messages[0][..2], &[0xB0, 1]); // modulation
    assert_eq!(messages[1][0], 0xE0); // pitch bend
    assert_eq!(&messages[2][..2], &[0xB0, 74]); // 汎用CC
}

#[test]
fn bag_covers_all_combinations_without_repeat() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    set_cc_number_74(&mut state);
    state.cycle_velocity(now);
    state.cycle_velocity(now); // Periodic
    state.cycle_modulation(now);
    state.cycle_modulation(now); // Periodic
    state.toggle_cc_periodic(now);

    // vel2 × mod2 × CC2 = 8通りを8tickで網羅し、次のbagでも再度網羅する
    for bag in 0..2u64 {
        let mut seen = HashSet::new();
        for tick in 1..=8 {
            let messages = state.poll_periodic(at_tick(now, bag * 8 + tick));
            assert_eq!(messages.len(), 2);
            assert_eq!(&messages[0][..2], &[0xB0, 1]); // modulation
            assert_eq!(&messages[1][..2], &[0xB0, 74]); // 汎用CC
            seen.insert((state.velocity(), messages[0][2], messages[1][2]));
        }
        assert_eq!(seen.len(), 8, "8通りの組み合わせが重複なく現れるはず");
    }
}

#[test]
fn master_clock_restarts_from_latest_digit_toggle() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    state.cycle_modulation(now);
    state.cycle_modulation(now); // Periodic(クロックはnow+250ms)
                                 // 300ms後にCC周期をON→クロックはそこから250ms後へ再スタート
    state.toggle_cc_periodic(now + Duration::from_millis(300));

    assert!(state
        .poll_periodic(now + Duration::from_millis(549))
        .is_empty());
    let messages = state.poll_periodic(now + Duration::from_millis(550));
    // 両系統が同一tickにまとまって送出される
    assert_eq!(messages.len(), 2);
    assert_eq!(&messages[0][..2], &[0xB0, 1]);
}

#[test]
fn digit_toggle_resets_bag_and_clock() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    for _ in 0..5 {
        state.cycle_pitch_bend(now); // Periodic(4-bag)
    }
    assert_eq!(state.combo_progress(), Some((0, 4)));
    assert!(!state.poll_periodic(at_tick(now, 1)).is_empty());
    assert!(!state.poll_periodic(at_tick(now, 2)).is_empty());
    assert_eq!(state.combo_progress(), Some((2, 4)));

    // 桁追加でbagとクロックを先頭から作り直す
    state.toggle_cc_periodic(now + Duration::from_millis(700));
    assert_eq!(state.combo_progress(), Some((0, 8)));
    assert!(state
        .poll_periodic(now + Duration::from_millis(949))
        .is_empty());
    assert_eq!(
        state.poll_periodic(now + Duration::from_millis(950)).len(),
        2
    );
    assert_eq!(state.combo_progress(), Some((1, 8)));
}

#[test]
fn combo_progress_wraps_after_bag_is_exhausted() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    assert_eq!(state.combo_progress(), None);
    for _ in 0..5 {
        state.cycle_pitch_bend(now); // Periodic(4-bag)
    }
    for tick in 1..=4u64 {
        let _ = state.poll_periodic(at_tick(now, tick));
        assert_eq!(state.combo_progress(), Some((tick as usize, 4)));
    }
    // 一巡したら再シャッフルして先頭から
    let _ = state.poll_periodic(at_tick(now, 5));
    assert_eq!(state.combo_progress(), Some((1, 4)));
}

#[test]
fn combo_progress_is_none_with_note_repeat_only() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    assert!(state.release(KEYBOARD_NOTES[0]).is_some());
    let _ = state.cycle_note_playback(now);
    assert!(state.poll_periodic(at_tick(now, 1)).is_empty());
    assert_eq!(state.combo_progress(), None);
}

#[test]
fn poll_periodic_skips_missed_cycles_after_long_stall() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    state.cycle_modulation(now);
    state.cycle_modulation(now);

    // 大幅遅延時は1発のみ送出し、次回はnow基準の250ms後へスナップ
    assert_eq!(state.poll_periodic(now + Duration::from_secs(10)).len(), 1);
    assert!(state
        .poll_periodic(now + Duration::from_millis(10_249))
        .is_empty());
    assert_eq!(
        state
            .poll_periodic(now + Duration::from_millis(10_250))
            .len(),
        1
    );
}

#[test]
fn take_note_off_messages_keeps_auto_send_modes_and_schedules_refresh() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    set_cc_number_74(&mut state);

    state.cycle_modulation(now); // On
    state.cycle_pitch_bend(now); // Max
    state.toggle_cc_periodic(now); // 127送信済み(bag先頭=127)
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    let _ = state.cycle_note_playback(now);

    // patch変更: note offのみ送出し、modeは維持される
    assert_eq!(
        state.take_note_off_messages(),
        vec![[0x80, 60, 0], [0x80, 60, 0]]
    );
    assert_eq!(state.modulation_mode(), ModulationMode::On);
    assert_eq!(state.pitch_bend_mode(), PitchBendMode::Max);
    assert!(state.cc_periodic_on());
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Repeat);

    // Ready復帰時に現在値+リトリガー和音を再送
    assert_eq!(
        state.take_pending_refresh_messages(now),
        vec![
            [0xB0, 1, 127],     // modulation ON
            [0xE0, 0x7F, 0x7F], // pitch bend +8191
            [0xB0, 74, 127],    // 汎用CC現在値
            [0x90, 60, 100],    // note repeat和音の再発音
        ]
    );
    assert!(state.take_pending_refresh_messages(now).is_empty());
}

#[test]
fn refresh_resends_current_periodic_values() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    state.cycle_modulation(now);
    state.cycle_modulation(now); // Periodic(0送信済み)
    let messages = state.poll_periodic(at_tick(now, 1));
    assert_eq!(messages.len(), 1);

    let _ = state.take_note_off_messages();
    // 周期中の現在値(最後のtickで送った値)を再送する
    assert_eq!(state.take_pending_refresh_messages(now), messages);
}

#[test]
fn take_reset_messages_clears_pending_refresh() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    state.cycle_modulation(now); // On
    let _ = state.take_note_off_messages();
    let _ = state.take_reset_messages();
    assert!(state.take_pending_refresh_messages(now).is_empty());
}
