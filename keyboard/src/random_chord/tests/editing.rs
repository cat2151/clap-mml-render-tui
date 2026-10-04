use super::*;

fn type_mml(screen: &mut KeyboardScreen<'_>, ctx: &KeyboardContext<'_>, value: &str) {
    screen.handle_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE), ctx);
    for _ in 0..screen.mml_input.value().chars().count() {
        screen.handle_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE), ctx);
    }
    for ch in value.chars() {
        screen.handle_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE), ctx);
    }
}

#[test]
fn editing_buffer_and_conversion_error_survive_a_random_boundary_then_cancel_or_confirm() {
    let load = PatchLoadState::Loading;
    let ctx = context(&load);
    for confirm in [false, true] {
        let mut screen = screen();
        screen.random_chord.enabled = true;
        screen.random_chord.catalog =
            ChordProgressionCatalog::from_json(r#"[{"degrees":"I-IV"}]"#).unwrap();
        let origin = Instant::now() + Duration::from_secs(60);
        screen.state.cycle_note_playback(origin);
        screen.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE), &ctx);
        type_mml(&mut screen, &ctx, "r");
        screen.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &ctx);
        let error = screen.mml_input.error().unwrap().to_string();
        let old_target = target(&screen);
        assert!(screen.random_chord_mode());
        assert_eq!(screen.mml_input.last_confirmed(), "ceg");
        assert_eq!(old_target, cmrt_chord::note_progression("ceg").unwrap());

        let boundary_step = old_target.len() as u64 * 8;
        for step in 1..=boundary_step {
            let deadline = origin + Duration::from_millis(250 * step);
            screen.apply_random_chord_progression(deadline - crate::periodic_timeline::LOOKAHEAD);
            screen.poll_random_chord_tick(deadline);
        }
        let boundary = origin + Duration::from_millis(250 * boundary_step);
        screen.apply_random_chord_progression(boundary);
        let generated = screen.mml_input.last_confirmed().to_string();
        assert_eq!(
            target(&screen),
            cmrt_chord::note_progression(&generated).unwrap()
        );
        assert_eq!(screen.mml_input.value(), "r");
        assert_eq!(screen.mml_input.error(), Some(error.as_str()));
        assert!(screen.random_chord_mode());

        if confirm {
            // 次の先読みを予約してから確定しても、手入力に自動更新が後から被らない。
            for step in 1..=16 {
                screen.poll_random_chord_tick(boundary + Duration::from_millis(250 * step));
            }
            assert!(screen.random_chord.pending_mml.is_some());
            type_mml(&mut screen, &ctx, "C-F");
            screen.handle_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE), &ctx);
            assert!(!screen.random_chord_mode());
            screen.apply_random_chord_progression(boundary + Duration::from_secs(10));
            assert_eq!(screen.mml_input.last_confirmed(), "C-F");
            assert_eq!(
                target(&screen),
                cmrt_chord::note_progression("C-F").unwrap()
            );
            assert!(screen.random_chord.pending_mml.is_none());
        } else {
            screen.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &ctx);
            assert!(screen.random_chord_mode());
            assert_eq!(screen.mml_input.last_confirmed(), generated);
            screen.handle_key(KeyEvent::new(KeyCode::Char('i'), KeyModifiers::NONE), &ctx);
            assert_eq!(screen.mml_input.value(), generated);
        }
    }
}
