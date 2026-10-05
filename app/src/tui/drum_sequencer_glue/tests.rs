use super::*;
use cmrt_tui_core::patch_load::{PatchCatalogSnapshot, PatchLoadMeasurement};
use ratatui::{backend::TestBackend, text::Span, Terminal};
use std::sync::Arc;

fn press(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ready() -> PatchLoadState {
    let measurements = [
        ("Drums/Kit.sfz", vec![36, 38]),
        ("Kits/909.sfz", vec![38, 42]),
    ]
    .into_iter()
    .map(|(name, notes)| {
        (
            name.to_string(),
            PatchLoadMeasurement {
                drum_kit: true,
                drum_kit_notes: Some(notes),
                ..Default::default()
            },
        )
    })
    .collect();
    let pairs = ["Drums/Kit.sfz", "Kits/909.sfz", "Pads/Warm Pad.fxp"]
        .into_iter()
        .map(|name| (name.to_string(), name.to_lowercase()))
        .collect();
    PatchLoadState::Ready(Arc::new(PatchCatalogSnapshot::new(
        pairs,
        Vec::new(),
        Vec::new(),
        Vec::new(),
        measurements,
    )))
}

fn app_with(state: PatchLoadState) -> TuiApp<'static> {
    let app = TuiApp::new_for_test(crate::tui::tests::test_config());
    *app.patch_load_state.lock().unwrap() = state;
    app
}

fn selected<'app, 'text>(app: &'app TuiApp<'text>) -> &'app PatchSelect<'text> {
    match app.drum_sequencer.selector.as_ref() {
        Some(DrumKitSelector::Select(select)) => select,
        _ => panic!("expected kit selector"),
    }
}

fn draw_selector(app: &TuiApp<'_>) -> String {
    let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
    terminal
        .draw(|frame| app.drum_sequencer.draw_selector(frame))
        .unwrap();
    let buffer = terminal.backend().buffer();
    (0..buffer.area.height)
        .map(|y| {
            let mut line = String::new();
            let mut x = 0;
            while x < buffer.area.width {
                let symbol = buffer[(x, y)].symbol();
                line.push_str(symbol);
                x += (Span::raw(symbol).width() as u16).max(1);
            }
            line
        })
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn confirmation_and_cancel_consume_keys_and_preserve_matrix_input() {
    let mut app = app_with(ready());
    let sink = Arc::new(cmrt_mml_overlay::RecordingSink::default());
    app.mml_overlay_sender = Some(cmrt_mml_overlay::MmlOverlaySender::with_recording_sink(
        Arc::clone(&sink),
        48_000.0,
    ));
    app.drum_sequencer
        .screen
        .set_kit("Drums/Kit.sfz".to_string(), Some(vec![36, 38]));
    for code in [KeyCode::Char('j'), KeyCode::Char('l'), KeyCode::Enter] {
        app.handle_drum_sequencer_key_event(press(code));
    }
    assert!(app.drum_sequencer.screen.cell_on(38, 1));

    assert_eq!(
        app.handle_drum_sequencer_key_event(press(KeyCode::Char('t'))),
        DrumSequencerAction::Continue
    );
    for code in [
        KeyCode::Down,
        KeyCode::Char(' '),
        KeyCode::Char('s'),
        KeyCode::Char('q'),
    ] {
        assert_eq!(
            app.handle_drum_sequencer_key_event(press(code)),
            DrumSequencerAction::Continue
        );
    }
    assert_eq!(
        app.handle_drum_sequencer_key_event(press(KeyCode::Esc)),
        DrumSequencerAction::Continue
    );
    assert!(!app.drum_sequencer.selector_open());
    assert_eq!(app.drum_sequencer.screen.kit_name(), Some("Drums/Kit.sfz"));
    assert!(app.drum_sequencer.screen.cell_on(38, 1));

    app.handle_drum_sequencer_key_event(press(KeyCode::Char('t')));
    app.handle_drum_sequencer_key_event(press(KeyCode::Down));
    assert_eq!(
        app.handle_drum_sequencer_key_event(press(KeyCode::Enter)),
        DrumSequencerAction::KitChanged
    );
    assert_eq!(app.drum_sequencer.screen.kit_name(), Some("Kits/909.sfz"));
    assert_eq!(app.drum_sequencer.screen.notes(), [38, 42]);
    assert_eq!(app.drum_sequencer.screen.cursor_note(), Some(38));
    assert_eq!(app.drum_sequencer.screen.cursor_step(), 1);
    assert!(
        app.drum_sequencer.screen.cell_on(38, 1),
        "confirm Enter must not toggle behind selector"
    );
    let mut release = press(KeyCode::Enter);
    release.kind = KeyEventKind::Release;
    app.handle_drum_sequencer_key_event(release);
    assert!(app.drum_sequencer.screen.cell_on(38, 1));
    assert!(!app.mml_overlay.is_open());
    assert!(
        sink.timeline_events().is_empty(),
        "selection must not audition"
    );
}

#[test]
fn loading_opens_only_kits_after_completion_and_escape_cancels_reservation() {
    let mut app = app_with(PatchLoadState::Loading);
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('t')));
    assert!(draw_selector(&app).contains("読み込み中"));
    assert_eq!(
        app.handle_drum_sequencer_key_event(press(KeyCode::Enter)),
        DrumSequencerAction::Continue
    );
    assert!(app.drum_sequencer.screen.kit_name().is_none());
    *app.patch_load_state.lock().unwrap() = ready();
    app.sync_drum_sequencer_catalog();
    assert!(selected(&app).drum_kit_only());
    assert_eq!(selected(&app).filtered_len(), 2);
    assert_eq!(selected(&app).filtered_display(1), Some("Kits/909.sfz"));
    app.handle_drum_sequencer_key_event(press(KeyCode::Esc));

    *app.patch_load_state.lock().unwrap() = PatchLoadState::Loading;
    app.handle_drum_sequencer_key_event(press(KeyCode::Char('t')));
    app.handle_drum_sequencer_key_event(press(KeyCode::Esc));
    *app.patch_load_state.lock().unwrap() = ready();
    app.sync_drum_sequencer_catalog();
    assert!(
        !app.drum_sequencer.selector_open(),
        "cancelled reservation must not reopen"
    );
}

#[test]
fn catalog_error_and_empty_kit_set_show_reasons_without_changing_kit() {
    for state in [
        PatchLoadState::Err("catalog fixture error".to_string()),
        PatchLoadState::ready(vec![(
            "Pads/Warm Pad.fxp".to_string(),
            "pads/warm pad.fxp".to_string(),
        )]),
        PatchLoadState::ready(Vec::new()),
    ] {
        let mut app = app_with(PatchLoadState::Loading);
        app.drum_sequencer
            .screen
            .set_kit("Old kit.sfz".to_string(), Some(vec![36]));
        app.handle_drum_sequencer_key_event(press(KeyCode::Enter));
        app.handle_drum_sequencer_key_event(press(KeyCode::Char('t')));
        *app.patch_load_state.lock().unwrap() = state;
        app.sync_drum_sequencer_catalog();
        let text = draw_selector(&app);
        assert!(text.contains("catalog fixture error") || text.contains("Drum kit がありません"));
        assert_eq!(
            app.handle_drum_sequencer_key_event(press(KeyCode::Enter)),
            DrumSequencerAction::Continue
        );
        app.handle_drum_sequencer_key_event(press(KeyCode::Esc));
        assert_eq!(app.drum_sequencer.screen.kit_name(), Some("Old kit.sfz"));
        assert!(app.drum_sequencer.screen.cell_on(36, 0));
    }
}
