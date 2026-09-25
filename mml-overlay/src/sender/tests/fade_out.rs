//! 行の演奏の fadeout（`MmlOverlaySender::fade_out_line`）。

use super::*;

fn line_with(patch: &str) -> (LivePatch, LineProgram) {
    let SenderCommandKind::PlayLine { program, .. } = line(1.0, false) else {
        unreachable!()
    };
    (LivePatch::new(Some(patch)), program)
}

fn fade_outs(sink: &FakeSink) -> Vec<(Vec<u8>, u32)> {
    sink.fade_outs.lock().unwrap().clone()
}

fn slow_pair() -> Arc<FakeSink> {
    Arc::new(FakeSink {
        standby_pair: true,
        prepare_delay: Duration::from_millis(300),
        ..FakeSink::default()
    })
}

/// 次の行の音色の準備（先読み）を worker が待っている最中でも、fadeout はその場で出る。
/// 宛先は前の行を鳴らした instance で、次の行が鳴った後はその instance に移る。
#[test]
fn the_fadeout_does_not_wait_for_the_next_line_to_load() {
    let sink = slow_pair();
    let sender = MmlOverlaySender::spawn(Arc::clone(&sink), 48_000.0);
    let (patch, program) = line_with("a.fxp");
    sender.play_line(patch, program);
    wait_until(|| sink.begins() == 1);

    let (patch, program) = line_with("b.fxp");
    sender.play_line(patch, program);
    std::thread::sleep(Duration::from_millis(50));
    let asked_at = Instant::now();
    assert_eq!(sender.fade_out_line(50), Ok(true));
    assert!(
        asked_at.elapsed() < Duration::from_millis(100),
        "準備の後ろに並んだ: {:?}",
        asked_at.elapsed()
    );
    assert_eq!(fade_outs(&sink), vec![(vec![0], 50)]);
    assert_eq!(
        sink.prepared.lock().unwrap().len(),
        1,
        "b の準備はまだ終わっていない"
    );

    // 準備を待っていた b は鳴らない。鳴らし直すと、読み込み済みの instance 1 で鳴る。
    let (patch, program) = line_with("b.fxp");
    sender.play_line(patch, program);
    wait_until(|| sink.begins() == 2);
    assert_eq!(sender.fade_out_line(50), Ok(true));
    assert_eq!(fade_outs(&sink), vec![(vec![0], 50), (vec![1], 50)]);
    assert_eq!(sink.stops(), 0, "全NoteOff は fadeout を段差で切る");
}

/// fadeout の後は、準備を待っていた行を鳴らさない（LIVE の候補から cache の候補へ移ったとき）。
#[test]
fn a_line_still_loading_is_dropped_by_the_fadeout() {
    let sink = slow_pair();
    let sender = MmlOverlaySender::spawn(Arc::clone(&sink), 48_000.0);
    let (patch, program) = line_with("a.fxp");
    sender.play_line(patch, program);
    wait_until(|| sink.begins() == 1);

    let (patch, program) = line_with("b.fxp");
    sender.play_line(patch, program);
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(sender.fade_out_line(50), Ok(true));
    wait_until(|| sink.prepared.lock().unwrap().len() == 2);
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(sink.begins(), 1);
}

/// fadeout は呼んだときだけ出る。行を続けて鳴らすだけ（MML overlay・Chord Chart の経路）では出ない。
#[test]
fn playing_lines_alone_never_fades() {
    let sink = Arc::new(FakeSink {
        standby_pair: true,
        ..FakeSink::default()
    });
    let sender = MmlOverlaySender::spawn(Arc::clone(&sink), 48_000.0);
    for (index, patch) in ["a.fxp", "b.fxp", "b.fxp"].into_iter().enumerate() {
        let (patch, program) = line_with(patch);
        sender.play_line(patch, program);
        wait_until(|| sink.begins() == index + 1);
    }
    assert!(fade_outs(&sink).is_empty());
}

/// 鳴らした行が無い、または止めた後なら、何も送らない。
#[test]
fn nothing_is_faded_without_a_sounding_line() {
    let sink = Arc::new(FakeSink::default());
    let sender = MmlOverlaySender::spawn(Arc::clone(&sink), 48_000.0);
    assert_eq!(sender.fade_out_line(50), Ok(false));

    let (patch, program) = line_with("a.fxp");
    sender.play_line(patch, program);
    wait_until(|| sink.begins() == 1);
    sender.stop();
    wait_until(|| sink.stops() == 1);
    assert_eq!(sender.fade_out_line(50), Ok(false));
    assert!(fade_outs(&sink).is_empty());
}
