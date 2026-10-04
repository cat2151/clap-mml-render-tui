use super::*;

#[test]
fn note_repeat_retriggers_last_chord_every_eight_master_ticks() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    // c e g を同時押し→全release。和音は保持される
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    assert!(state.press(KEYBOARD_NOTES[2]).is_some());
    assert!(state.press(KEYBOARD_NOTES[4]).is_some());
    assert!(state.release(KEYBOARD_NOTES[0]).is_some());
    assert!(state.release(KEYBOARD_NOTES[2]).is_some());
    assert!(state.release(KEYBOARD_NOTES[4]).is_some());

    // auto(voicing未判定はrepeat扱い): 即座に和音を発音
    assert_eq!(
        state.cycle_note_playback(now),
        vec![[0x90, 60, 100], [0x90, 64, 100], [0x90, 67, 100]]
    );
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Auto);
    // repeatへ移っても同じ和音を鳴らし続ける
    assert!(state.cycle_note_playback(now).is_empty());
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Repeat);
    // 1～7tick目は鳴らし続け、8tick目にoff+onを同時送信
    for tick in 1..8 {
        assert!(state.poll_periodic(at_tick(now, tick)).is_empty());
    }
    let retrigger = vec![
        [0x80, 60, 0],
        [0x80, 64, 0],
        [0x80, 67, 0],
        [0x90, 60, 100],
        [0x90, 64, 100],
        [0x90, 67, 100],
    ];
    assert_eq!(state.poll_periodic(at_tick(now, 8)), retrigger);
    for tick in 9..16 {
        assert!(state.poll_periodic(at_tick(now, tick)).is_empty());
    }
    assert_eq!(state.poll_periodic(at_tick(now, 16)), retrigger);
    // arpへ移るとrepeatの和音を止め、arp先頭音を鳴らす
    assert_eq!(
        state.cycle_note_playback(now),
        vec![[0x80, 60, 0], [0x80, 64, 0], [0x80, 67, 0], [0x90, 60, 100],]
    );
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Arp);
    let _ = state.cycle_note_playback(now);
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Off);
    assert!(state
        .poll_periodic(now + Duration::from_secs(10))
        .is_empty());
}

#[test]
fn note_repeat_advances_through_the_chord_progression_every_eight_ticks() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    state.replace_repeat_chords(vec![vec![60, 64, 67], vec![62, 65, 69]], now, false);

    assert_eq!(
        state.cycle_note_playback(now),
        vec![[0x90, 60, 100], [0x90, 64, 100], [0x90, 67, 100]]
    );
    for tick in 1..8 {
        assert!(state.poll_periodic(at_tick(now, tick)).is_empty());
    }
    assert_eq!(
        state.poll_periodic(at_tick(now, 8)),
        vec![
            [0x80, 60, 0],
            [0x80, 64, 0],
            [0x80, 67, 0],
            [0x90, 62, 100],
            [0x90, 65, 100],
            [0x90, 69, 100],
        ]
    );
    for tick in 9..16 {
        assert!(state.poll_periodic(at_tick(now, tick)).is_empty());
    }
    assert_eq!(
        state.poll_periodic(at_tick(now, 16)),
        vec![
            [0x80, 62, 0],
            [0x80, 65, 0],
            [0x80, 69, 0],
            [0x90, 60, 100],
            [0x90, 64, 100],
            [0x90, 67, 100],
        ]
    );
}

#[test]
fn repeat_requests_one_new_progression_only_at_the_full_cycle_boundary() {
    for mode in [NotePlaybackMode::Repeat, NotePlaybackMode::Auto] {
        let mut state = KeyboardState::default();
        let origin = Instant::now();
        state.set_detected_voicing(PatchVoicing::Poly);
        state.replace_repeat_chords(vec![vec![60], vec![62, 65]], origin, false);
        state.cycle_note_playback(origin);
        if mode == NotePlaybackMode::Repeat {
            state.cycle_note_playback(origin);
        }
        let generation = state.periodic_generation();
        let mut calls = 0;
        for step in 1..=24 {
            let at = at_tick(origin, step);
            state.apply_scheduled_progression(at - Duration::from_millis(200));
            let tick = state
                .poll_periodic_tick_with_progression(at, |boundary| {
                    assert_eq!(boundary, at);
                    calls += 1;
                    Some(vec![vec![67, 71, 74]])
                })
                .unwrap();
            assert_eq!(tick.at, at);
            assert_eq!(calls, usize::from(step >= 16) + usize::from(step >= 24));
            if step == 8 {
                assert_eq!(
                    tick.messages,
                    vec![[0x80, 60, 0], [0x90, 62, 100], [0x90, 65, 100]]
                );
            }
            if step == 16 {
                assert_eq!(
                    tick.messages,
                    vec![
                        [0x80, 62, 0],
                        [0x80, 65, 0],
                        [0x90, 67, 100],
                        [0x90, 71, 100],
                        [0x90, 74, 100],
                    ]
                );
                assert_eq!(
                    state.repeat_chords().len(),
                    2,
                    "lookahead keeps the applied target"
                );
                assert_eq!(state.repeat_chords_at(at).len(), 1);
                assert_eq!(state.sounding_position(at).unwrap().chord_index, 0);
            }
            assert_eq!(state.periodic_generation(), generation);
            assert_eq!(state.periodic_anchor(), Some(origin));
        }
    }
}

#[test]
fn off_controller_ticks_never_request_a_progression() {
    let mut state = KeyboardState::default();
    let origin = Instant::now();
    state.replace_repeat_chords(vec![vec![60]], origin, false);
    state.toggle_cc_periodic(origin);
    for step in 1..=24 {
        let tick = state
            .poll_periodic_tick_with_progression(at_tick(origin, step), |_| {
                panic!("off must not request a progression")
            })
            .unwrap();
        assert!(tick.messages.iter().all(|message| message[0] == 0xB0));
        assert_eq!(state.repeat_chords()[0][0].midi_note, 60);
    }
}

#[test]
fn note_repeat_does_nothing_without_chord() {
    let mut state = KeyboardState::default();
    assert!(state.cycle_note_playback(Instant::now()).is_empty());
    assert_eq!(state.note_playback_mode(), NotePlaybackMode::Off);
}

#[test]
fn note_repeat_uses_current_velocity_on_each_retrigger() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    assert!(state.release(KEYBOARD_NOTES[0]).is_some());
    state.cycle_velocity(now); // Accent(127)
    assert_eq!(state.cycle_note_playback(now), vec![[0x90, 60, 127]]);
    for tick in 1..8 {
        assert!(state.poll_periodic(at_tick(now, tick)).is_empty());
    }
    assert_eq!(
        state.poll_periodic(at_tick(now, 8)),
        vec![[0x80, 60, 0], [0x90, 60, 127]]
    );
}

#[test]
fn note_repeat_counts_resynchronized_master_clock_ticks() {
    let mut state = KeyboardState::default();
    let now = Instant::now();
    assert!(state.press(KEYBOARD_NOTES[0]).is_some());
    assert!(state.release(KEYBOARD_NOTES[0]).is_some());
    let _ = state.cycle_note_playback(now);
    assert!(state.poll_periodic(at_tick(now, 1)).is_empty()); // repeatの1tick目

    state.cycle_modulation(now + Duration::from_millis(300)); // On
    state.cycle_modulation(now + Duration::from_millis(300)); // Periodicへ入りクロックを再同期
    assert!(state
        .poll_periodic(now + Duration::from_millis(500))
        .is_empty());
    // 再同期後のtickを数え、独立した2000ms時刻ではリトリガーしない
    for elapsed_ms in [550, 800, 1050, 1300, 1550, 1800] {
        let messages = state.poll_periodic(now + Duration::from_millis(elapsed_ms));
        assert_eq!(messages.len(), 1);
        assert_eq!(&messages[0][..2], &[0xB0, 1]);
    }
    assert!(state
        .poll_periodic(now + Duration::from_millis(2000))
        .is_empty());
    let messages = state.poll_periodic(now + Duration::from_millis(2050));
    // 8tick目では値変更→note off→note onの順で同一tickにまとまる
    assert_eq!(messages.len(), 3);
    assert_eq!(&messages[0][..2], &[0xB0, 1]);
    assert_eq!(messages[1], [0x80, 60, 0]);
    assert_eq!(messages[2], [0x90, 60, 100]);
}
