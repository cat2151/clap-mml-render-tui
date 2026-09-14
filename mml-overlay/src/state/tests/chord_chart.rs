use super::*;

fn patches() -> Vec<PatchCatalogEntry> {
    ["Leads/Lead 1.fxp", "Pads/Pad 1.fxp"]
        .into_iter()
        .map(|patch| PatchCatalogEntry::from_display(patch.to_string()))
        .collect()
}

fn chord_chart_overlay(key: &str, initial_text: &str) -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        input_mode: MmlOverlayInputMode::SingleLine,
        single_line_flow: SingleLineFlow::Modal,
        initial_text: initial_text.to_string(),
        syntax: MmlOverlaySyntax::ChordChart(ChordChartPreviewContext {
            key_token: Some(key.to_string()),
        }),
        patch_catalog: PatchCatalogSnapshot::Ready(patches()),
        ..MmlOverlayContext::default()
    });
    overlay
}

fn played(action: MmlOverlayAction) -> (PatchChange, crate::line_play::LineProgram) {
    let MmlOverlayAction::PlayLine { patch, program } = action else {
        panic!("expected line playback, got {action:?}");
    };
    (patch, program)
}

fn pitches(program: &crate::line_play::LineProgram) -> Vec<u8> {
    program
        .events()
        .iter()
        .filter(|event| event.message[0] == NOTE_ON)
        .map(|event| event.message[1])
        .collect()
}

fn turn_on_repeat(overlay: &mut MmlOverlay<'_>, now: Instant) {
    overlay.handle_key(ctrl(KeyCode::Char('l')), now);
    overlay.handle_key(press(KeyCode::Char(' ')), now);
    overlay.handle_key(press(KeyCode::Enter), now);
    assert!(overlay.play_settings().repeat);
}

#[test]
fn chord_chart_typing_uses_the_song_key() {
    let now = Instant::now();
    let mut in_c = chord_chart_overlay("Key=C", "");
    let mut in_g = chord_chart_overlay("Key=G", "");

    assert_sent_pitches(&type_chars(&mut in_c, "II", now), &[62, 66, 69]);
    assert_sent_pitches(&type_chars(&mut in_g, "II", now), &[69, 73, 76]);
}

#[test]
fn moving_between_equal_chords_replays_by_visible_source_span() {
    let now = Instant::now();
    let mut overlay = chord_chart_overlay("Key=C", "I I");

    assert_sent_pitches(
        &overlay.handle_key(press(KeyCode::Left), now),
        &[60, 64, 67],
    );
    assert_sent_pitches(
        &overlay.handle_key(press(KeyCode::Left), now),
        &[60, 64, 67],
    );
}

#[test]
fn invalid_chord_chart_input_is_silent_instead_of_falling_back_to_mml() {
    let mut overlay = chord_chart_overlay("Key=G", "");

    let action = type_chars(&mut overlay, "cde", Instant::now());

    assert_eq!(action, MmlOverlayAction::Continue);
    assert!(overlay.sounding().is_empty());

    let (_, program) = played(overlay.handle_key(ctrl(KeyCode::Char(' ')), Instant::now()));
    assert!(program.is_silent());
    assert!(matches!(overlay.line_status(), LineStatus::Error(_)));
}

#[test]
fn ctrl_space_replays_the_unwrapped_progression_with_the_song_key() {
    let now = Instant::now();
    let mut overlay = chord_chart_overlay("Key=G", "I V");

    let (patch, program) = played(overlay.handle_key(ctrl(KeyCode::Char(' ')), now));
    let (_, daw_cell) = crate::line_play::chord_line_events("I V", "Key=G", "", "");

    assert_eq!(patch, PatchChange::Keep);
    assert_eq!(pitches(&program), vec![67, 71, 74, 74, 78, 81]);
    assert!(program.performance.loop_seconds > daw_cell.loop_seconds);
}

#[test]
fn patch_cursor_previews_the_current_chord_with_the_candidate_patch() {
    let now = Instant::now();
    let mut overlay = chord_chart_overlay("Key=G", "II");
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    let MmlOverlayAction::SetPatch {
        patch,
        notes: Some(notes),
    } = overlay.handle_key(press(KeyCode::Down), now)
    else {
        panic!("expected candidate chord preview");
    };
    assert_eq!(patch.as_deref(), Some("Pads/Pad 1.fxp"));
    assert_eq!(
        notes
            .messages
            .iter()
            .map(|message| message[1])
            .collect::<Vec<_>>(),
        vec![69, 73, 76]
    );
    assert!(notes.duration > Duration::ZERO);
}

#[test]
fn space_in_the_patch_selector_replays_the_progression_with_the_candidate_patch() {
    let now = Instant::now();
    let mut overlay = chord_chart_overlay("Key=G", "I V");
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    let (patch, program) = played(overlay.handle_key(press(KeyCode::Char(' ')), now));

    assert_eq!(
        patch,
        PatchChange::Switch(Some("Leads/Lead 1.fxp".to_string()))
    );
    assert_eq!(pitches(&program), vec![67, 71, 74, 74, 78, 81]);
}

#[test]
fn invalid_chord_chart_input_does_not_use_the_patch_fallback_note() {
    let now = Instant::now();
    let mut overlay = chord_chart_overlay("Key=G", "cde");
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    assert_eq!(
        overlay.handle_key(press(KeyCode::Down), now),
        MmlOverlayAction::SetPatch {
            patch: Some("Pads/Pad 1.fxp".to_string()),
            notes: None,
        }
    );
}

#[test]
fn only_confirm_changes_the_chord_chart_patch_and_cancel_restores_it() {
    let now = Instant::now();
    let mut overlay = chord_chart_overlay("Key=C", "I");
    overlay.set_restored_patch(Some("Leads/Lead 1.fxp".to_string()));

    overlay.handle_key(ctrl(KeyCode::Char('t')), now);
    overlay.handle_key(press(KeyCode::Down), now);
    assert_eq!(
        overlay.handle_key(press(KeyCode::Esc), now),
        MmlOverlayAction::SetPatch {
            patch: Some("Leads/Lead 1.fxp".to_string()),
            notes: None,
        }
    );
    assert_eq!(overlay.patch(), Some("Leads/Lead 1.fxp"));

    overlay.handle_key(ctrl(KeyCode::Char('t')), now);
    overlay.handle_key(press(KeyCode::Down), now);
    assert_eq!(
        overlay.handle_key(press(KeyCode::Enter), now),
        MmlOverlayAction::Continue
    );
    assert_eq!(overlay.patch(), Some("Pads/Pad 1.fxp"));
}

#[test]
fn patch_selector_replay_keeps_the_progression_context_and_shape() {
    let now = Instant::now();
    let mut overlay = chord_chart_overlay("Key=G", "I V");
    turn_on_repeat(&mut overlay, now);
    overlay.handle_key(ctrl(KeyCode::Char('t')), now);

    let (patch, program) = played(overlay.handle_key(press(KeyCode::Down), now));
    let (_, daw_cell) = crate::line_play::chord_line_events("I V", "Key=G", "", "");

    assert_eq!(
        patch,
        PatchChange::Switch(Some("Pads/Pad 1.fxp".to_string()))
    );
    assert_eq!(pitches(&program), vec![67, 71, 74, 74, 78, 81]);
    assert!(program.repeat);
    assert!(program.performance.loop_seconds > daw_cell.loop_seconds);
}
