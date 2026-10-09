use super::*;

fn step_loop(hits: Vec<StepHit>) -> SenderCommandKind {
    SenderCommandKind::PlayStepLoop {
        patch: LivePatch::new(Some("kit.sfz")),
        step_loop: StepLoop {
            loop_seconds: 0.4,
            hits,
            horizon_seconds: 0.25,
        },
    }
}

fn hit(seconds: f64, note: u8) -> StepHit {
    StepHit {
        seconds,
        note,
        velocity: 127,
        gate_seconds: 0.4,
    }
}

fn on_seconds(sink: &FakeSink, note: u8) -> Vec<f64> {
    sink.timeline_events
        .lock()
        .unwrap()
        .iter()
        .filter(|event| event.message == [NOTE_ON, note, 127])
        .map(|event| event.timeline_seconds)
        .collect()
}

/// worker が自分で先を積み続け、張り直さずに打点の差し替えを受け、止めたら積むのをやめる。
#[test]
fn the_worker_keeps_one_timeline_running_and_takes_hit_updates() {
    let sink = Arc::new(FakeSink::default());
    let harness = Harness::spawn(Arc::clone(&sink));
    // 空で始めても時計は進み、状態は終わりの無い演奏として公開される。
    harness.send(1, step_loop(Vec::new()));
    wait_until(|| harness.status.lock().unwrap().line_playback().is_some());
    let playback = harness.status.lock().unwrap().line_playback().unwrap();
    assert_eq!((playback.command_id(), playback.ends_at()), (1, None));

    harness
        .tx
        .send(WorkerMessage::StepLoop(StepLoopEdit::Hits(vec![hit(
            0.0, 36,
        )])))
        .unwrap();
    wait_until(|| on_seconds(&sink, 36).len() >= 3);
    let ons = on_seconds(&sink, 36);
    for pair in ons.windows(2) {
        assert!((pair[1] - pair[0] - 0.4).abs() < 1e-9, "{ons:?}");
    }
    assert_eq!(sink.begins(), 1, "updates never re-begin the timeline");
    assert_eq!(
        harness.status.lock().unwrap().command_id(),
        1,
        "an update is not a command"
    );

    harness.send(2, SenderCommandKind::Stop);
    wait_until(|| sink.stops() > 0);
    std::thread::sleep(Duration::from_millis(50));
    let after_stop = sink.timeline_seconds().len();
    std::thread::sleep(Duration::from_millis(500));
    assert_eq!(sink.timeline_seconds().len(), after_stop);
    // 止めた後の差し替えはループを生き返らせない。
    harness
        .tx
        .send(WorkerMessage::StepLoop(StepLoopEdit::Hits(vec![hit(
            0.0, 38,
        )])))
        .unwrap();
    std::thread::sleep(Duration::from_millis(300));
    assert!(on_seconds(&sink, 38).is_empty());
}

/// 単発は周回を待たずに積まれ、timeline を張り直さない。
#[test]
fn a_shot_reaches_the_sink_without_re_beginning_the_timeline() {
    let sink = Arc::new(FakeSink::default());
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(1, step_loop(Vec::new()));
    wait_until(|| harness.status.lock().unwrap().line_playback().is_some());
    harness
        .tx
        .send(WorkerMessage::StepLoop(StepLoopEdit::Shot(StepShot {
            note: 40,
            velocity: 127,
            gate_seconds: 0.1,
        })))
        .unwrap();
    wait_until(|| !on_seconds(&sink, 40).is_empty());
    assert_eq!(on_seconds(&sink, 40).len(), 1);
    assert_eq!(sink.begins(), 1);
}
