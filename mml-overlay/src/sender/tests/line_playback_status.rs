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
            patch: LivePatch::new(Some("ready.sfz")),
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

/// `solo_violin.mid` と同じ 16469 件。上限が 4096 だった頃は先頭だけを積んで止まり、
/// 演奏区間も公開されなかった。
#[test]
fn a_long_smf_line_is_sent_whole_and_publishes_an_interval() {
    const EVENTS: usize = 16_469;
    let sink = Arc::new(FakeSink::default());
    let harness = Harness::spawn(Arc::clone(&sink));
    let events = (0..EVENTS)
        .map(|index| cmrt_chord::TimedMidiEvent {
            seconds: index as f64 * 0.023,
            message: [NOTE_ON, (index % 40) as u8 + 40, 100],
        })
        .collect::<Vec<_>>();
    let loop_seconds = EVENTS as f64 * 0.023;

    harness.send(
        1,
        SenderCommandKind::PlayLine {
            patch: LivePatch::new(Some("ready.sfz")),
            program: LineProgram::once(LinePerformance {
                events: events.clone(),
                loop_seconds,
            }),
        },
    );
    wait_until(|| playback(&harness).is_some());

    let sent = sink.timeline_events.lock().unwrap().clone();
    assert_eq!(sent.len(), EVENTS);
    for (sent, original) in sent.iter().zip(&events) {
        assert_eq!(sent.message, original.message);
    }
    assert!(sent
        .windows(2)
        .all(|pair| pair[0].timeline_seconds < pair[1].timeline_seconds));
    let playback = playback(&harness).unwrap();
    assert_eq!(
        playback.ends_at().unwrap() - playback.started_at(),
        Duration::from_secs_f64(loop_seconds)
    );
}
