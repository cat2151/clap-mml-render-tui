use super::*;

fn on_seconds(sink: &RecordingSink, note: u8) -> Vec<f64> {
    sink.timeline_events()
        .iter()
        .filter(|event| event.message == [0x90, note, 127])
        .map(|event| event.timeline_seconds)
        .collect()
}

fn line(app: &TuiApp<'_>) -> cmrt_mml_overlay::MmlOverlayLinePlayback {
    app.mml_overlay_sender
        .as_ref()
        .unwrap()
        .status()
        .line_playback()
        .unwrap()
}

#[test]
fn an_empty_loop_keeps_running_takes_edits_without_restarting_and_toggles_off() {
    let sink = Arc::new(RecordingSink::default());
    let (_dirs, mut app) = app_with_sink(Arc::clone(&sink));
    play(&mut app, &sink);
    let playback = line(&app);
    assert_eq!(playback.ends_at(), None, "the loop has no end");
    assert_eq!(sink.prepared(), vec![LivePatch::new(Some("Kit.sfz"))]);

    // 空で始めたループへ、step 5（0.5 秒）の 36 を足す。
    for _ in 0..4 {
        app.dispatch_drum_sequencer_key_event(key(KeyCode::Char('l')));
    }
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Enter));
    // 先読み 0.05 秒を除いた周内の位置。
    let in_cycle = |seconds: f64| (seconds - 0.05).rem_euclid(2.0);
    let on_step_5 = |seconds: &f64| (in_cycle(*seconds) - 0.5).abs() < 1e-9;
    wait_until(|| on_seconds(&sink, 36).iter().any(on_step_5));
    // ON にした瞬間の単発が先に鳴り、周回の打点は step 5 に来る。先頭から鳴らし直していない。
    let ons = on_seconds(&sink, 36);
    assert!(!on_step_5(&ons[0]), "the shot comes first: {ons:?}");
    assert_eq!(ons.iter().filter(|seconds| on_step_5(seconds)).count(), 1);
    assert_eq!(sink.timelines(), 1, "editing never re-begins the timeline");
    assert_eq!(sink.stops(), 0);
    assert_eq!(line(&app).command_id(), playback.command_id());

    // もう一度 Shift+P で止まり、それ以降は積まれない。
    app.dispatch_drum_sequencer_key_event(preview());
    wait_until(|| sink.stops() == 1);
    assert!(app.drum_sequencer.loop_hits.is_none());
    std::thread::sleep(Duration::from_millis(50));
    let after_stop = sink.timeline_events().len();
    std::thread::sleep(Duration::from_millis(400));
    assert_eq!(sink.timeline_events().len(), after_stop);
    assert!(
        app.drum_sequencer.screen.cell_on(36, 4),
        "stopping keeps input"
    );
}

#[test]
fn the_playhead_follows_the_published_start_wraps_and_clears_when_replaced() {
    let sink = Arc::new(RecordingSink::default());
    let (_dirs, mut app) = app_with_sink(Arc::clone(&sink));
    play(&mut app, &sink);
    let started = line(&app).started_at();
    for (offset, expected) in [
        (0.0, Some(0)),
        (0.13, Some(1)),
        (1.99, Some(15)),
        (2.0, Some(0)),
        (2.26, Some(2)),
    ] {
        app.sync_drum_sequencer_playhead(started + Duration::from_secs_f64(offset));
        assert_eq!(app.drum_sequencer.screen.playhead(), expected, "{offset}");
    }
    // 周の先頭が鳴る前（開始準備中）は出さない。
    app.sync_drum_sequencer_playhead(started - Duration::from_millis(1));
    assert_eq!(app.drum_sequencer.screen.playhead(), None);

    // 別の演奏に置き換わったら、画面が再生中のつもりでも出さない。
    let sender = app.mml_overlay_sender.as_ref().unwrap();
    let replaced = sender.stop();
    wait_until(|| sender.status().command_id() == replaced);
    app.sync_drum_sequencer_playhead(started + Duration::from_secs_f64(0.5));
    assert_eq!(app.drum_sequencer.screen.playhead(), None);

    // 停止したら消える。
    app.drum_sequencer.screen.set_playhead(Some(3));
    app.dispatch_drum_sequencer_key_event(preview());
    assert_eq!(app.drum_sequencer.screen.playhead(), None);
}

#[test]
fn opening_the_kit_list_stops_the_loop_without_unknown_or_empty_kits_starting_one() {
    let sink = Arc::new(RecordingSink::default());
    let (_dirs, mut app) = app_with_sink(Arc::clone(&sink));
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Enter));
    play(&mut app, &sink);
    *app.patch_load_state.lock().unwrap() = crate::tui::PatchLoadState::Loading;
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Char('t')));
    assert!(app.drum_sequencer.selector_open());
    wait_until(|| sink.stops() == 1);
    assert!(app.drum_sequencer.loop_hits.is_none());
    app.dispatch_drum_sequencer_key_event(key(KeyCode::Esc));

    for notes in [None, Some(Vec::new())] {
        app.drum_sequencer
            .screen
            .set_kit("Other.sfz".to_string(), notes, Vec::new(), Vec::new());
        app.dispatch_drum_sequencer_key_event(preview());
        assert!(app.drum_sequencer.loop_hits.is_none());
    }
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(
        sink.timelines(),
        1,
        "unknown / empty kits never start a loop"
    );
}
