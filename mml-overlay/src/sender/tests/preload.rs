//! 鳴っていない bank への先読み（`MmlOverlaySender::preload`）。

use super::*;

const LITE: &str = "lite.sfz";
const FULL: &str = "full.sfz";

fn program() -> LineProgram {
    let SenderCommandKind::PlayLine { program, .. } = line(1.0, false) else {
        unreachable!()
    };
    program
}

fn patch(name: &str) -> LivePatch {
    LivePatch::new(Some(name))
}

fn last_timeline_instance(sink: &RecordingSink) -> u8 {
    sink.timeline_events()
        .last()
        .expect("a timeline event was sent")
        .instance_id
}

/// lite を鳴らしてから full を先読みした sender。先読みはまだ終わっていない。
fn lite_playing_while_full_loads() -> (Arc<RecordingSink>, MmlOverlaySender) {
    let sink = Arc::new(RecordingSink::default());
    let sender = MmlOverlaySender::with_recording_sink(Arc::clone(&sink), 48_000.0);
    sender.prepare(patch(LITE));
    sender.play_line(patch(LITE), program());
    sender.preload(patch(FULL));
    wait_until(|| sender.status().preload().is_some());
    (sink, sender)
}

#[test]
fn a_preload_goes_to_the_other_bank_without_blocking_the_current_patch() {
    let (sink, sender) = lite_playing_while_full_loads();
    assert_eq!(
        sink.operations(),
        vec![
            SinkOperation::Prepare,
            SinkOperation::Timeline,
            SinkOperation::Preload { instance_id: 1 },
        ]
    );
    assert_eq!(sink.prepared(), vec![patch(LITE)]);
    assert_eq!(sink.preloads(), vec![patch(FULL)]);
    assert_eq!(
        sender.status().preload(),
        Some(&MmlOverlayPreload::Loading(patch(FULL)))
    );
    assert!(!sender.status().is_loading(), "先読みで画面を塞がない");

    // 先読みの完了を進めないまま、lite の行を鳴らし直す。待たずに張られる。
    sender.play_line(patch(LITE), program());
    wait_until(|| sink.timelines() == 2);
    assert_eq!(sink.prepared(), vec![patch(LITE)]);
    assert_eq!(last_timeline_instance(&sink), 0);
    assert!(!sender.status().is_loading());
}

#[test]
fn a_finished_preload_plays_on_the_other_bank_without_loading() {
    let (sink, sender) = lite_playing_while_full_loads();
    sink.finish_preload();
    wait_until(|| sender.status().preload() == Some(&MmlOverlayPreload::Ready(patch(FULL))));

    sender.play_line(patch(FULL), program());
    wait_until(|| sink.timelines() == 2);
    assert_eq!(sink.prepared(), vec![patch(LITE)], "Prepare を出さない");
    assert_eq!(last_timeline_instance(&sink), 1);

    // 戻るときも読み直さない（lite は instance 0 に残っている）。
    sender.play_line(patch(LITE), program());
    wait_until(|| sink.timelines() == 3);
    assert_eq!(sink.prepared(), vec![patch(LITE)]);
    assert_eq!(last_timeline_instance(&sink), 0);
}

#[test]
fn a_line_with_another_patch_waits_for_the_preload_to_settle() {
    let (sink, sender) = lite_playing_while_full_loads();
    sender.play_line(patch("other.sfz"), program());
    wait_until(|| sender.status().is_loading());
    std::thread::sleep(Duration::from_millis(50));
    assert_eq!(sink.timelines(), 1, "先読みの決着前に鳴らさない");
    assert_eq!(sink.prepared(), vec![patch(LITE)], "同時に 2 件読まない");

    sink.finish_preload();
    wait_until(|| sink.timelines() == 2);
    assert_eq!(sink.prepared(), vec![patch(LITE), patch("other.sfz")]);
    assert_eq!(sink.preloads(), vec![patch(FULL)]);
    assert!(!sender.status().is_loading());
    // full を読んだ bank へ other を読んだので、full はもう使えない。
    assert_eq!(sender.status().preload(), None);
}

#[test]
fn a_preload_of_a_ready_patch_does_not_load_again() {
    let sink = Arc::new(RecordingSink::default());
    let sender = MmlOverlaySender::with_recording_sink(Arc::clone(&sink), 48_000.0);
    sender.prepare(patch(LITE));
    sender.preload(patch(LITE));
    wait_until(|| sender.status().preload().is_some());
    assert_eq!(
        sender.status().preload(),
        Some(&MmlOverlayPreload::Ready(patch(LITE)))
    );
    assert!(sink.preloads().is_empty());
}

/// 先読みの決着を待っている最中に sender を畳んでも、先読みの完了を待たずに返る。
/// 待つと、先読みの長さ（数十秒）だけアプリの終了が塞がる。
#[test]
fn dropping_while_a_line_waits_for_the_preload_does_not_wait_for_it() {
    let (sink, sender) = lite_playing_while_full_loads();
    sender.play_line(patch("other.sfz"), program());
    wait_until(|| sender.status().is_loading());

    let (done_tx, done_rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        drop(sender);
        let _ = done_tx.send(());
    });

    assert!(
        done_rx.recv_timeout(Duration::from_secs(2)).is_ok(),
        "先読みを終えないと drop が返らない"
    );
    assert_eq!(
        sink.prepared(),
        vec![patch(LITE)],
        "畳むときに音色を読まない"
    );
}
