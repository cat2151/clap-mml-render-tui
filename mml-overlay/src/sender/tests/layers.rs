use std::collections::BTreeMap;

use cmrt_chord::TimedMidiEvent;

use super::*;

fn event(seconds: f64, message: [u8; 3]) -> TimedMidiEvent {
    TimedMidiEvent { seconds, message }
}

fn layer(
    instance_id: u8,
    patch: &str,
    loop_seconds: f64,
    events: Vec<TimedMidiEvent>,
) -> LineLayer {
    LineLayer {
        instance_id,
        patch: Some(patch.to_string()),
        performance: LinePerformance {
            events,
            loop_seconds,
        },
    }
}

fn chord_and_bass() -> Vec<LineLayer> {
    vec![
        layer(
            0,
            "chord.sfz",
            1.0,
            vec![
                event(0.0, [NOTE_ON, 60, 100]),
                event(1.0, [NOTE_OFF, 60, 0]),
            ],
        ),
        layer(
            1,
            "bass.sfz",
            2.0,
            vec![
                event(0.0, [NOTE_ON, 36, 110]),
                event(1.0, [NOTE_ON, 38, 110]),
            ],
        ),
    ]
}

#[test]
fn layers_share_one_timeline_and_keep_instance_patch_state() {
    let sink = Arc::new(FakeSink::default());
    let harness = Harness::spawn(Arc::clone(&sink));

    harness.send(
        1,
        SenderCommandKind::PlayLayers {
            layers: chord_and_bass(),
        },
    );
    wait_until(|| {
        harness
            .status
            .lock()
            .unwrap()
            .line_playback()
            .is_some_and(|playback| playback.command_id() == 1)
    });

    assert_eq!(sink.begins(), 1);
    assert_eq!(
        *sink.prepared.lock().unwrap(),
        vec![
            (0, Some("chord.sfz".to_string())),
            (1, Some("bass.sfz".to_string()))
        ]
    );
    let events = sink.timeline_events.lock().unwrap().clone();
    assert!(events.iter().any(|event| event.instance_id == 0));
    assert!(events.iter().any(|event| event.instance_id == 1));
    assert!(events
        .iter()
        .all(|event| event.timeline_id == events[0].timeline_id));
    let simultaneous = events
        .iter()
        .filter(|event| (event.timeline_seconds - 1.05).abs() < f64::EPSILON)
        .map(|event| event.message[0])
        .collect::<Vec<_>>();
    assert_eq!(simultaneous, vec![NOTE_OFF, NOTE_ON]);

    let playback = harness.status.lock().unwrap().line_playback().unwrap();
    assert_eq!(
        playback.ends_at().unwrap() - playback.started_at(),
        Duration::from_secs(2)
    );

    harness.send(
        2,
        SenderCommandKind::PlayLayers {
            layers: chord_and_bass(),
        },
    );
    wait_until(|| sink.begins() == 2);
    assert_eq!(sink.prepared.lock().unwrap().len(), 2);
}

#[test]
fn normal_line_and_layers_supersede_each_other_in_both_orders() {
    let sink = Arc::new(FakeSink::default());
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(1, line(1.0, false));
    wait_until(|| sink.begins() == 1);

    let layers = vec![
        layer(0, "ready.sfz", 1.0, vec![event(0.0, [NOTE_ON, 60, 100])]),
        layer(1, "bass.sfz", 1.0, vec![event(0.0, [NOTE_ON, 36, 100])]),
    ];
    harness.send(2, SenderCommandKind::PlayLayers { layers });
    wait_until(|| sink.begins() == 2);
    harness.send(3, line(1.0, false));
    wait_until(|| {
        harness
            .status
            .lock()
            .unwrap()
            .line_playback()
            .is_some_and(|playback| playback.command_id() == 3)
    });

    // layers へ移るときは stop_all、行へ移るときは timeline の張り直しが前の音を止める。
    assert_eq!(sink.stops(), 1);
    assert_eq!(sink.begins(), 3);
    assert_eq!(
        *sink.prepared.lock().unwrap(),
        vec![
            (0, Some("ready.sfz".to_string())),
            (1, Some("bass.sfz".to_string()))
        ]
    );
    assert_eq!(
        sink.timeline_events
            .lock()
            .unwrap()
            .last()
            .unwrap()
            .instance_id,
        MML_OVERLAY_INSTANCE
    );

    harness.send(4, notes("ready.sfz", 64, Duration::from_millis(50)));
    wait_until(|| event_time(&sink, NOTE_ON, 64).is_some());
    assert!(sink
        .midi
        .lock()
        .unwrap()
        .iter()
        .all(|record| record.instance_id == MML_OVERLAY_INSTANCE));
}

#[test]
fn silent_layers_stop_the_previous_timeline_without_starting_another() {
    let sink = Arc::new(FakeSink::default());
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(
        1,
        SenderCommandKind::PlayLayers {
            layers: chord_and_bass(),
        },
    );
    wait_until(|| sink.begins() == 1);

    harness.send(2, SenderCommandKind::PlayLayers { layers: Vec::new() });
    wait_until(|| sink.stops() == 1);

    assert_eq!(sink.begins(), 1);
    assert_eq!(harness.status.lock().unwrap().line_playback(), None);
}

#[test]
fn explicit_stop_hard_stops_a_layered_timeline() {
    let sink = Arc::new(FakeSink::default());
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(
        1,
        SenderCommandKind::PlayLayers {
            layers: chord_and_bass(),
        },
    );
    wait_until(|| sink.begins() == 1);

    harness.send(2, SenderCommandKind::Stop);
    wait_until(|| sink.stops() == 1);

    assert_eq!(harness.status.lock().unwrap().line_playback(), None);
}

#[test]
fn shutdown_hard_stops_a_layered_timeline() {
    let sink = Arc::new(FakeSink::default());
    let (tx, rx) = mpsc::channel();
    let latest = Arc::new(AtomicU64::new(1));
    let status = Arc::new(Mutex::new(MmlOverlaySenderStatus::default()));
    let worker_latest = Arc::clone(&latest);
    let worker_status = Arc::clone(&status);
    let worker_sink = Arc::clone(&sink);
    let worker = std::thread::spawn(move || {
        run_sender(
            rx,
            worker_sink,
            48_000.0,
            worker_latest,
            worker_status,
            SoundingLines::default(),
        );
    });
    tx.send(SenderCommand {
        id: 1,
        queued_at: Instant::now(),
        kind: SenderCommandKind::PlayLayers {
            layers: chord_and_bass(),
        },
    })
    .unwrap();
    wait_until(|| sink.begins() == 1);

    latest.store(2, Ordering::Release);
    tx.send(SenderCommand {
        id: 2,
        queued_at: Instant::now(),
        kind: SenderCommandKind::Shutdown,
    })
    .unwrap();
    worker.join().unwrap();

    assert_eq!(sink.stops(), 1);
    assert_eq!(status.lock().unwrap().line_playback(), None);
}

#[test]
fn bass_prepare_failure_still_sends_the_chord_layer() {
    let sink = Arc::new(FakeSink {
        prepare_errors: BTreeMap::from([(1, "bass prepare failed".to_string())]),
        ..FakeSink::default()
    });
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(
        1,
        SenderCommandKind::PlayLayers {
            layers: chord_and_bass(),
        },
    );
    wait_until(|| sink.begins() == 1);

    let events = sink.timeline_events.lock().unwrap().clone();
    assert!(!events.is_empty());
    assert!(events.iter().all(|event| event.instance_id == 0));
    assert_eq!(
        harness.status.lock().unwrap().prepare_error(),
        Some("bass prepare failed")
    );
}

#[test]
fn chord_prepare_failure_keeps_the_whole_preview_silent() {
    let sink = Arc::new(FakeSink {
        prepare_errors: BTreeMap::from([(0, "chord prepare failed".to_string())]),
        ..FakeSink::default()
    });
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(
        1,
        SenderCommandKind::PlayLayers {
            layers: chord_and_bass(),
        },
    );
    wait_until(|| harness.status.lock().unwrap().prepare_error().is_some());

    assert_eq!(sink.begins(), 0);
    assert!(sink.timeline_events.lock().unwrap().is_empty());
}
