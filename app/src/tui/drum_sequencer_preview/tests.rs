use super::*;
use crate::screen_switch::PrimaryScreen;
use cmrt_mml_overlay::{MmlOverlaySender, RecordingSink, SinkOperation};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}
fn preview() -> KeyEvent {
    key(KeyCode::Char('P'))
}

fn wait_until(mut ready: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !ready() {
        assert!(Instant::now() < deadline, "sender condition timed out");
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn app_with_sink(sink: Arc<RecordingSink>) -> TuiApp<'static> {
    let mut app = TuiApp::new_for_test(crate::tui::tests::test_config());
    app.mml_overlay_sender = Some(MmlOverlaySender::with_recording_sink(sink, 48_000.0));
    app.switch_to_primary_screen(PrimaryScreen::DrumSequencer, None);
    app.drum_sequencer
        .screen
        .set_kit("Kit.sfz".to_string(), Some(vec![36, 38]));
    app
}

fn play(app: &mut TuiApp<'_>, sink: &RecordingSink) {
    let before = sink.timelines();
    app.dispatch_drum_sequencer_key_event(preview());
    wait_until(|| {
        sink.timelines() > before
            && app
                .mml_overlay_sender
                .as_ref()
                .unwrap()
                .status()
                .line_playback()
                .is_some()
    });
}

#[test]
fn timeline_keeps_each_cell_concurrent_notes_and_final_off_without_filters() {
    let mut screen = DrumSequencerScreen::default();
    screen.set_kit("kit".to_string(), Some(vec![36, 38, 42]));
    screen.handle_key_event(key(KeyCode::Enter));
    screen.handle_key_event(key(KeyCode::Char('j')));
    screen.handle_key_event(key(KeyCode::Enter));
    for _ in 0..15 {
        screen.handle_key_event(key(KeyCode::Char('l')));
    }
    screen.handle_key_event(key(KeyCode::Char('k')));
    screen.handle_key_event(key(KeyCode::Enter));
    for _ in 0..2 {
        screen.handle_key_event(key(KeyCode::Char('j')));
    }
    screen.handle_key_event(key(KeyCode::Enter));
    screen.set_kit("kit subset".to_string(), Some(vec![36, 38]));
    let program = preview_program(&screen);
    assert!(!program.repeat);
    assert!(!program.filters.modulation && !program.filters.velocity);
    assert_eq!(program.performance.loop_seconds, 3.875);
    let events: Vec<_> = program
        .events()
        .iter()
        .map(|event| (event.seconds, event.message))
        .collect();
    assert_eq!(
        events,
        vec![
            (0.0, [0x90, 36, 127]),
            (0.0, [0x90, 38, 127]),
            (1.875, [0x90, 36, 127]),
            (2.0, [0x80, 36, 0]),
            (2.0, [0x80, 38, 0]),
            (3.875, [0x80, 36, 0]),
        ]
    );
    assert!(
        screen.cell_on(42, 15),
        "hidden input survives without being sent"
    );
    screen.set_kit("unknown".to_string(), None);
    assert!(preview_program(&screen).is_silent());
}

#[test]
fn preview_restarts_once_and_empty_only_stops_without_loading() {
    let sink = Arc::new(RecordingSink::default());
    let mut app = app_with_sink(Arc::clone(&sink));
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Enter));
    assert!(
        sink.operations().is_empty(),
        "editing and screen entry are silent"
    );
    play(&mut app, &sink);
    let first = sink.timeline_events();
    assert_eq!(first.len(), 2);
    assert_eq!(first[0].message, [0x90, 36, 127]);
    assert!((first[1].timeline_seconds - first[0].timeline_seconds - 2.0).abs() < 1e-9);
    let interval = app
        .mml_overlay_sender
        .as_ref()
        .unwrap()
        .status()
        .line_playback()
        .unwrap();
    assert_eq!(
        interval.ends_at().unwrap() - interval.started_at(),
        Duration::from_secs(2)
    );
    let before = sink.operations().len();
    play(&mut app, &sink);
    assert!(
        sink.operations()[before..].starts_with(&[SinkOperation::Stop, SinkOperation::Timeline])
    );
    assert_eq!(sink.prepared(), vec![LivePatch::new(Some("Kit.sfz"))]);
    assert_eq!(
        sink.timeline_events().len(),
        4,
        "no modulation or repeat events"
    );
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Enter));
    let stops = sink.stops();
    app.dispatch_drum_sequencer_key_event(preview());
    wait_until(|| sink.stops() > stops);
    assert_eq!(sink.timelines(), 2);
    assert_eq!(sink.prepared().len(), 1, "empty phrase does not prepare");
}

#[test]
fn loading_stops_old_queue_and_departure_cancels_pending_preview() {
    let sink = Arc::new(RecordingSink::default());
    let mut app = app_with_sink(Arc::clone(&sink));
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Enter));
    play(&mut app, &sink);
    let sender = app.mml_overlay_sender.as_ref().unwrap();
    sender.preload(LivePatch::new(Some("Background.sfz")));
    wait_until(|| !sink.preloads().is_empty());
    app.drum_sequencer
        .screen
        .set_kit("NewKit.sfz".to_string(), Some(vec![36]));
    let stops = sink.stops();
    app.dispatch_drum_sequencer_key_event(preview());
    wait_until(|| {
        app.mml_overlay_sender
            .as_ref()
            .unwrap()
            .status()
            .is_loading()
    });
    assert!(
        sink.stops() > stops,
        "stop happens before async preparation"
    );
    assert_eq!(
        sink.timelines(),
        1,
        "no timeline is sent to old kit while loading"
    );
    let lines = crate::tui::ui::tests::render_lines(&mut app, 120, 30).join("\n");
    assert!(lines
        .chars()
        .filter(|ch| !ch.is_whitespace())
        .collect::<String>()
        .contains("音源"));
    assert!(app.sound_startup_wait.is_some());
    app.switch_to_primary_screen(PrimaryScreen::Notepad, None);
    sink.finish_preload();
    wait_until(|| {
        app.mml_overlay_sender
            .as_ref()
            .unwrap()
            .status()
            .line_playback()
            .is_none()
            && !app
                .mml_overlay_sender
                .as_ref()
                .unwrap()
                .status()
                .is_loading()
    });
    assert_eq!(
        sink.timelines(),
        1,
        "cancelled load must not start a late preview"
    );
    app.switch_to_primary_screen(PrimaryScreen::DrumSequencer, None);
    assert!(app.drum_sequencer.screen.cell_on(36, 0));
    assert_eq!(sink.timelines(), 1, "roundtrip does not auto play");
}

#[test]
fn kit_reconfirmation_overlay_quit_and_shutdown_stop_existing_preview() {
    let sink = Arc::new(RecordingSink::default());
    let mut app = app_with_sink(Arc::clone(&sink));
    *app.patch_load_state.lock().unwrap() = crate::tui::PatchLoadState::Loading;
    // All keys are consumed by the selector, including global shortcuts and q/P.
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Enter));
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Char('t')));
    assert!(!app.can_open_screen_switch_menu());
    for k in [
        preview(),
        key(KeyCode::Char('q')),
        KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL),
        KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL),
    ] {
        assert!(!app.dispatch_drum_sequencer_key_event(k));
    }
    assert!(sink.operations().is_empty());
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Esc));
    use cmrt_tui_core::patch_load::{PatchCatalogSnapshot, PatchLoadMeasurement};
    *app.patch_load_state.lock().unwrap() =
        crate::tui::PatchLoadState::Ready(Arc::new(PatchCatalogSnapshot::new(
            vec![("Kit.sfz".to_string(), "kit.sfz".to_string())],
            Vec::new(),
            Vec::new(),
            Vec::new(),
            [(
                "Kit.sfz".to_string(),
                PatchLoadMeasurement {
                    drum_kit: true,
                    drum_kit_notes: Some(vec![36, 38]),
                    ..Default::default()
                },
            )]
            .into_iter()
            .collect(),
        )));
    play(&mut app, &sink);
    let stops = sink.stops();
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Char('t')));
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Enter));
    wait_until(|| sink.stops() > stops);
    assert!(
        app.drum_sequencer.screen.cell_on(36, 0),
        "reconfirmation consumes Enter"
    );
    assert_eq!(sink.timelines(), 1);
    play(&mut app, &sink);
    let stops = sink.stops();
    assert!(app.try_open_mml_overlay(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL)));
    wait_until(|| sink.stops() > stops);
    app.handle_mml_overlay_key_event(key(KeyCode::Esc));
    let timelines = sink.timelines();
    assert_eq!(timelines, 2, "closing overlay cannot resume Drum preview");
    play(&mut app, &sink);
    let stops = sink.stops();
    assert!(app.dispatch_drum_sequencer_key_event(key(KeyCode::Char('q'))));
    wait_until(|| sink.stops() > stops);
    play(&mut app, &sink);
    let stops = sink.stops();
    drop(app);
    assert!(
        sink.stops() > stops,
        "sender shutdown clears scheduled events"
    );
}

#[test]
fn shift_p_conventions_are_press_only_and_menu_roundtrip_keeps_cells() {
    let sink = Arc::new(RecordingSink::default());
    let mut app = app_with_sink(Arc::clone(&sink));
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Enter));
    for (code, modifiers, expected) in [
        (KeyCode::Char('P'), KeyModifiers::NONE, true),
        (KeyCode::Char('P'), KeyModifiers::SHIFT, true),
        (KeyCode::Char('p'), KeyModifiers::SHIFT, true),
        (KeyCode::Char('p'), KeyModifiers::NONE, false),
        (KeyCode::Char('P'), KeyModifiers::CONTROL, false),
    ] {
        assert_eq!(is_preview_key(KeyEvent::new(code, modifiers)), expected);
    }
    for kind in [KeyEventKind::Repeat, KeyEventKind::Release] {
        let mut k = preview();
        k.kind = kind;
        app.dispatch_drum_sequencer_key_event(k);
    }
    assert!(sink.operations().is_empty());
    assert!(!app.uses_textarea_cursor() && !app.uses_mouse_capture());
    app.switch_to_primary_screen(PrimaryScreen::Notepad, None);
    assert!(
        app.try_open_screen_switch_menu(KeyEvent::new(KeyCode::Char('g'), KeyModifiers::CONTROL))
    );
    let target = app
        .handle_screen_switch_menu_key(key(KeyCode::Char('s')))
        .unwrap();
    assert_eq!(target, PrimaryScreen::DrumSequencer);
    app.switch_to_primary_screen(target, None);
    assert!(app.drum_sequencer.screen.cell_on(36, 0));
    assert_eq!(sink.timelines(), 0);
    let text = crate::tui::ui::tests::render_lines(&mut app, 120, 24).join("\n");
    assert!(text.contains("Drum Sequencer") && text.contains("Shift+P:preview"));
}

#[test]
fn failed_patch_preparation_reports_reason_and_keeps_input() {
    let sink = Arc::new(RecordingSink::failing_prepare("fixture kit load failure"));
    let mut app = app_with_sink(Arc::clone(&sink));
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Enter));
    app.dispatch_drum_sequencer_key_event(preview());
    wait_until(|| {
        app.mml_overlay_sender
            .as_ref()
            .unwrap()
            .status()
            .prepare_error()
            .is_some()
    });
    let text = crate::tui::ui::tests::render_lines(&mut app, 120, 24).join("\n");
    assert!(text.contains("fixture kit load failure"));
    assert!(app.drum_sequencer.screen.cell_on(36, 0));
    assert_eq!(sink.timelines(), 0);
    assert!(app.sound_startup_wait.is_none());
}
