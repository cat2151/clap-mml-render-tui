use super::*;

fn playback(harness: &Harness) -> Option<MmlOverlayLinePlayback> {
    harness.status.lock().unwrap().line_playback()
}

#[test]
fn a_line_interval_starts_only_after_slow_preparation_and_enqueue() {
    let sink = Arc::new(FakeSink {
        prepare_delay: Duration::from_millis(80),
        timeline_delay: Duration::from_millis(80),
        ..FakeSink::default()
    });
    let harness = Harness::spawn(Arc::clone(&sink));
    let requested_at = Instant::now();

    harness.send(1, line(0.25, false));
    wait_until(|| harness.status.lock().unwrap().is_loading());
    assert_eq!(playback(&harness), None, "load 中は演奏区間を公開しない");
    wait_until(|| sink.begins() == 1);
    assert_eq!(playback(&harness), None, "timeline 送信中も公開しない");
    wait_until(|| playback(&harness).is_some());

    let playback = playback(&harness).unwrap();
    assert_eq!(playback.command_id(), 1);
    assert!(playback.started_at() >= requested_at + Duration::from_millis(140));
    assert_eq!(
        playback.ends_at().unwrap() - playback.started_at(),
        Duration::from_millis(250)
    );
    assert!(playback.is_sounding_at(playback.started_at()));
    assert!(!playback.is_sounding_at(playback.ends_at().unwrap()));
}

#[test]
fn a_line_with_failed_preparation_never_publishes_an_interval() {
    let sink = Arc::new(FakeSink {
        prepare_error: Some("prepare failed".to_string()),
        ..FakeSink::default()
    });
    let harness = Harness::spawn(Arc::clone(&sink));

    harness.send(1, line(0.25, false));
    wait_until(|| harness.status.lock().unwrap().prepare_error().is_some());

    assert_eq!(sink.begins(), 0);
    assert_eq!(playback(&harness), None);
}

#[test]
fn an_already_ready_patch_publishes_the_new_command_interval() {
    let sink = Arc::new(FakeSink::default());
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(1, line(0.1, false));
    wait_until(|| playback(&harness).is_some());

    harness.send(2, line(0.4, false));
    wait_until(|| playback(&harness).is_some_and(|value| value.command_id() == 2));

    let playback = playback(&harness).unwrap();
    assert_eq!(playback.command_id(), 2);
    assert_eq!(
        playback.ends_at().unwrap() - playback.started_at(),
        Duration::from_millis(400)
    );
    assert_eq!(sink.prepared.lock().unwrap().len(), 1);
}

#[test]
fn failed_or_silent_lines_do_not_publish_an_interval() {
    let sink = Arc::new(FakeSink {
        begin_error: Some("timeline failed".to_string()),
        ..FakeSink::default()
    });
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(1, line(0.25, false));
    wait_until(|| sink.begins() == 1);
    assert_eq!(playback(&harness), None);

    harness.send(
        2,
        SenderCommandKind::PlayLine {
            patch: Some("ready.sfz".to_string()),
            program: LineProgram::silent(),
        },
    );
    wait_until(|| harness.status.lock().unwrap().command_id() == 2);
    assert_eq!(playback(&harness), None);
}

#[test]
fn stop_clears_the_previous_line_interval() {
    let sink = Arc::new(FakeSink::default());
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(1, line(1.0, false));
    wait_until(|| playback(&harness).is_some());

    harness.send(2, SenderCommandKind::Stop);
    wait_until(|| harness.status.lock().unwrap().command_id() == 2);

    assert_eq!(playback(&harness), None);
}

#[test]
fn a_line_superseded_during_load_never_publishes_an_interval() {
    let sink = Arc::new(FakeSink {
        prepare_delay: Duration::from_millis(80),
        ..FakeSink::default()
    });
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(1, line(1.0, false));
    wait_until(|| harness.status.lock().unwrap().is_loading());

    harness.send(2, SenderCommandKind::Stop);
    wait_until(|| harness.status.lock().unwrap().command_id() == 2);

    assert_eq!(sink.begins(), 0, "supersede された行を timeline へ積まない");
    assert_eq!(playback(&harness), None);
}
