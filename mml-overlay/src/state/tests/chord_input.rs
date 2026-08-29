use super::*;

fn chord_overlay(preview: Option<ChordPreviewContext>) -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        input_mode: MmlOverlayInputMode::SingleLine,
        syntax: MmlOverlaySyntax::Chord(preview),
        ..MmlOverlayContext::default()
    });
    overlay
}

fn preview_context() -> ChordPreviewContext {
    ChordPreviewContext {
        chord_init: "key:G".to_string(),
        track_directive: "close".to_string(),
        mml_prefix: String::new(),
        target_label: "T2".to_string(),
    }
}

#[test]
fn typing_a_degree_uses_the_chord_preview_context_immediately() {
    let mut overlay = chord_overlay(Some(preview_context()));

    let action = type_chars(&mut overlay, "II", Instant::now());

    assert_sent_pitches(&action, &[69, 73, 76]);
    assert_eq!(overlay.sounding(), [69, 73, 76]);
}

#[test]
fn a_chord_input_without_a_playback_track_stays_editable_but_silent() {
    let mut overlay = chord_overlay(None);

    let action = type_chars(&mut overlay, "II", Instant::now());

    assert_eq!(action, MmlOverlayAction::Continue);
    assert_eq!(overlay.value(), "II");
    assert!(overlay.sounding().is_empty());
}

#[test]
fn replaying_a_chord_line_uses_the_same_context_as_typing() {
    let mut overlay = chord_overlay(Some(preview_context()));
    type_chars(&mut overlay, "II", Instant::now());

    let action = overlay.handle_key(ctrl(KeyCode::Char(' ')), Instant::now());
    let MmlOverlayAction::PlayLine { program, .. } = action else {
        panic!("expected line playback, got {action:?}");
    };
    let pitches = program
        .events()
        .iter()
        .filter(|event| event.message[0] == NOTE_ON)
        .map(|event| event.message[1])
        .collect::<Vec<_>>();
    assert_eq!(pitches, vec![69, 73, 76]);
}

#[test]
fn committing_chord_input_never_opens_the_transfer_confirmation() {
    let mut overlay = chord_overlay(Some(preview_context()));
    type_chars(&mut overlay, "II", Instant::now());

    let action = overlay.handle_key(press(KeyCode::Enter), Instant::now());

    assert_eq!(
        action,
        MmlOverlayAction::Commit {
            line: "II".to_string(),
            close: false,
        }
    );
}

#[test]
fn chord_input_does_not_open_the_mml_phrase_history() {
    let mut overlay = chord_overlay(Some(preview_context()));

    let action = overlay.handle_key(ctrl(KeyCode::Char('o')), Instant::now());

    assert_eq!(action, MmlOverlayAction::Continue);
    assert!(overlay.history_select().is_none());
}
