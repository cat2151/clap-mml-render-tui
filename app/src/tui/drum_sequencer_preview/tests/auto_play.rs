use super::*;

/// 自動開始で 1 本目のループが sender に届くまで待つ。
fn pump_until_playing(app: &mut TuiApp<'_>, sink: &RecordingSink) {
    let before = sink.timelines();
    app.pump_drum_sequencer_auto_play();
    assert!(app.drum_sequencer.loop_hits.is_some(), "auto play starts");
    wait_until(|| sink.timelines() > before);
}

#[test]
fn auto_play_starts_once_and_shift_p_stop_sticks_until_the_kit_list_closes() {
    let sink = Arc::new(RecordingSink::default());
    let (_dirs, mut app) = app_with_sink(Arc::clone(&sink));
    pump_until_playing(&mut app, &sink);
    let timelines = sink.timelines();
    app.pump_drum_sequencer_auto_play();
    assert_eq!(
        sink.timelines(),
        timelines,
        "a running loop is not restarted"
    );

    app.dispatch_drum_sequencer_key_event(preview());
    assert!(app.drum_sequencer.loop_hits.is_none());
    app.pump_drum_sequencer_auto_play();
    assert!(
        app.drum_sequencer.loop_hits.is_none(),
        "Shift+P stop is not undone"
    );

    *app.patch_load_state.lock().unwrap() = crate::tui::PatchLoadState::Loading;
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Char('t')));
    app.pump_drum_sequencer_auto_play();
    assert!(
        app.drum_sequencer.loop_hits.is_none(),
        "the open kit list stays quiet"
    );
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Esc));
    pump_until_playing(&mut app, &sink);
}

#[test]
fn auto_play_waits_for_kit_notes_and_for_the_mml_overlay_to_close() {
    let sink = Arc::new(RecordingSink::default());
    let (_dirs, mut app) = app_with_sink(Arc::clone(&sink));
    app.drum_sequencer
        .screen
        .set_kit("Kit.sfz".to_string(), None, Vec::new(), Vec::new());
    app.pump_drum_sequencer_auto_play();
    assert!(app.drum_sequencer.loop_hits.is_none(), "unknown notes wait");
    app.drum_sequencer.screen.set_kit(
        "Kit.sfz".to_string(),
        Some(vec![36]),
        Vec::new(),
        Vec::new(),
    );
    pump_until_playing(&mut app, &sink);

    assert!(app.try_open_mml_overlay(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::CONTROL)));
    assert!(app.drum_sequencer.loop_hits.is_none());
    app.pump_drum_sequencer_auto_play();
    assert!(
        app.drum_sequencer.loop_hits.is_none(),
        "the overlay owns the instrument"
    );
    app.handle_mml_overlay_key_event(key(KeyCode::Esc));
    pump_until_playing(&mut app, &sink);
}
