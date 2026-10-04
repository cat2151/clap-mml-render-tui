use super::*;
use crate::periodic_timeline::{TimelineSend, LOOKAHEAD};

fn playing(mode: NotePlaybackMode) -> (KeyboardScreen<'static>, Instant, u64) {
    let mut screen = screen();
    screen.random_chord.enabled = true;
    screen.random_chord.catalog =
        ChordProgressionCatalog::from_json(r#"[{"degrees":"I-IV"}]"#).unwrap();
    let origin = Instant::now() + Duration::from_secs(60);
    while screen.state.note_playback_mode() != mode {
        screen.state.cycle_note_playback(origin);
    }
    screen
        .periodic_timeline
        .plan(origin, Vec::new(), None, || 1);
    let ticks = if mode == NotePlaybackMode::Repeat {
        target(&screen).len() as u64 * 8
    } else {
        target(&screen)
            .iter()
            .map(|chord| chord.len() as u64 * 2)
            .sum()
    };
    (screen, origin, ticks)
}

fn advance(screen: &mut KeyboardScreen<'_>, origin: Instant, ticks: u64) {
    for step in 1..=ticks {
        let deadline = origin + Duration::from_millis(250 * step);
        screen.apply_random_chord_progression(deadline - LOOKAHEAD);
        let tick = screen.poll_random_chord_tick(deadline);
        screen
            .periodic_timeline
            .plan(deadline - LOOKAHEAD, Vec::new(), tick, || {
                panic!("existing timeline")
            });
    }
}

#[test]
fn disabling_before_and_after_lookahead_keeps_the_last_progression_and_cancels_future_midi() {
    for mode in [NotePlaybackMode::Repeat, NotePlaybackMode::Arp] {
        for after_lookahead in [false, true] {
            let (mut screen, origin, ticks) = playing(mode);
            let old_target = target(&screen);
            advance(&mut screen, origin, ticks - u64::from(!after_lookahead));
            let boundary = origin + Duration::from_millis(250 * ticks);
            let disabled_at = boundary - LOOKAHEAD + Duration::from_millis(1);

            screen.disable_random_chord_mode(disabled_at);

            assert!(!screen.random_chord_mode());
            assert!(screen.random_chord.pending_mml.is_none());
            assert_eq!(screen.mml_input.last_confirmed(), "ceg");
            assert_eq!(target(&screen), old_target);
            assert_eq!(screen.state.note_playback_mode(), mode);
            let refresh = screen.state.take_pending_refresh_messages(disabled_at);
            let sends = screen
                .periodic_timeline
                .plan(disabled_at, refresh, None, || 2);
            if after_lookahead {
                assert_eq!(sends[0], TimelineSend::Begin(2));
                assert!(sends.contains(&TimelineSend::Immediate(vec![[0x90, 60, 100]])));
            } else {
                assert!(sends.is_empty());
            }
            for step in 1..=ticks * 2 {
                let now = disabled_at + Duration::from_millis(250 * step);
                screen.apply_random_chord_progression(now);
                screen.poll_random_chord_tick(now);
                assert!(screen.random_chord.pending_mml.is_none());
                assert_eq!(target(&screen), old_target);
                assert_eq!(screen.mml_input.last_confirmed(), "ceg");
            }
        }
    }
}

#[test]
fn off_before_and_after_lookahead_discards_updates_even_with_periodic_cc() {
    for after_lookahead in [false, true] {
        let (mut screen, origin, ticks) = playing(NotePlaybackMode::Arp);
        screen.state.toggle_cc_periodic(origin);
        let old_target = target(&screen);
        advance(&mut screen, origin, ticks - u64::from(!after_lookahead));
        let boundary = origin + Duration::from_millis(250 * ticks);
        let stopped_at = boundary - LOOKAHEAD + Duration::from_millis(1);

        let messages = screen.cycle_keyboard_note_playback(stopped_at);
        assert!(messages.iter().all(|message| message[0] != 0x90));
        screen.send_after_cancel(messages);
        let sends = screen
            .periodic_timeline
            .plan(stopped_at, Vec::new(), None, || 2);
        assert_eq!(sends, vec![TimelineSend::Begin(2)]);
        assert!(screen.random_chord_mode());
        assert_eq!(screen.state.note_playback_mode(), NotePlaybackMode::Off);
        for step in 1..=ticks * 2 {
            let now = stopped_at + Duration::from_millis(250 * step);
            screen.apply_random_chord_progression(now);
            let tick = screen.poll_random_chord_tick(now).unwrap();
            assert!(tick.messages.iter().all(|message| message[0] == 0xB0));
            assert_eq!(screen.mml_input.last_confirmed(), "ceg");
            assert_eq!(target(&screen), old_target);
            assert!(screen.random_chord.pending_mml.is_none());
        }
    }
}

#[test]
fn patch_refresh_and_exit_do_not_apply_a_canceled_progression() {
    let load = PatchLoadState::Loading;
    let ctx = context(&load);
    for leaving in [false, true] {
        let (mut screen, origin, ticks) = playing(NotePlaybackMode::Arp);
        let old_target = target(&screen);
        advance(&mut screen, origin, ticks);
        let boundary = origin + Duration::from_millis(250 * ticks);
        assert!(screen.random_chord.pending_mml.is_some());

        if leaving {
            screen.finish();
            assert!(screen.periodic_sending_stopped());
            screen.pump_periodic(boundary + Duration::from_secs(1), "2026-10-04");
            assert!(screen.periodic_sending_stopped());
            screen.resume(&ctx);
        } else {
            screen.apply_patch_selection(Some("next.fxp".to_string()), &ctx);
        }
        screen.apply_random_chord_progression(boundary + Duration::from_secs(2));
        assert!(screen.random_chord_mode());
        assert_eq!(screen.mml_input.last_confirmed(), "ceg");
        assert_eq!(target(&screen), old_target);
        assert!(screen.random_chord.pending_mml.is_none());
        let refresh = screen.state.take_pending_refresh_messages(boundary);
        assert_eq!(refresh, vec![[0x90, 60, 100]]);
        assert_eq!(
            screen
                .state
                .sounding_position(boundary)
                .unwrap()
                .chord_index,
            0
        );
    }
}

#[test]
fn session_and_share_observe_the_same_progression_before_and_at_the_boundary() {
    let (mut screen, origin, ticks) = playing(NotePlaybackMode::Repeat);
    advance(&mut screen, origin, ticks);
    let boundary = origin + Duration::from_millis(250 * ticks);
    let next_mml = screen.random_chord.pending_mml.as_ref().unwrap().1.clone();
    for (now, expected) in [
        (boundary - Duration::from_millis(1), "ceg"),
        (boundary, next_mml.as_str()),
    ] {
        let saved = screen.session_state_at(now);
        assert_eq!(saved.mml, expected);
        assert_eq!(
            cmrt_chord::note_progression(expected).unwrap(),
            saved.repeat_chords
        );
        assert!(crate::share_command(&saved).contains(&format!(
            "-m {}",
            if expected == "ceg" {
                expected.to_string()
            } else {
                format!("\"{expected}\"")
            }
        )));
        let restored = KeyboardState::from_session(saved.clone());
        assert_eq!(
            restored.session_state(saved.mml).repeat_chords,
            saved.repeat_chords
        );
    }
}
