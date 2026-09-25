//! 準備の失敗理由を status に残し、画面が読めるまで持ち越す。

use super::*;

/// 準備に失敗したら、**`loading` は必ず下ろし、理由は status に残す。**
///
/// 画面はこの 2 つで「音が鳴るまで」の overlay を閉じ、消えた理由を出す
/// （`app/src/tui/sound_startup_overlay.rs`）。理由を持ち帰れないと、
/// overlay が黙って消えて音も鳴らない状態になる。
#[test]
fn a_failed_preparation_lowers_loading_and_keeps_its_reason() {
    let sink = Arc::new(FakeSink {
        prepare_error: Some("play server が起動できません".to_string()),
        ..FakeSink::default()
    });
    let harness = Harness::spawn(Arc::clone(&sink));

    harness.send(
        1,
        SenderCommandKind::Prepare {
            patch: LivePatch::new(Some("lead.fxp")),
        },
    );
    wait_until(|| harness.status.lock().unwrap().prepare_error().is_some());

    let status = harness.status.lock().unwrap().clone();
    assert!(
        !status.is_loading(),
        "失敗しても loading は下ろすこと（overlay が出っぱなしになる）"
    );
    assert_eq!(status.prepare_error(), Some("play server が起動できません"));
}

/// 成功したら理由は消える。古い失敗が居座ると、鳴っているのに理由が出続ける。
#[test]
fn a_successful_preparation_clears_the_previous_reason() {
    let sink = Arc::new(FakeSink::default());
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.status.lock().unwrap().prepare_error = Some("stale".to_string());

    harness.send(
        1,
        SenderCommandKind::Prepare {
            patch: LivePatch::default(),
        },
    );
    wait_until(|| !sink.prepared.lock().unwrap().is_empty());
    wait_until(|| harness.status.lock().unwrap().prepare_error().is_none());
}

/// 失敗の直後に別の command が来ても、理由は消えない。
///
/// 失敗すると画面はまず overlay を閉じ、そのあと理由を読む。読む前に
/// `Stop` の 1 つでも挟まると理由が消える作りだと、「黙って消えて音も鳴らない」に戻る。
#[test]
fn the_reason_survives_the_next_command() {
    let sink = Arc::new(FakeSink {
        prepare_error: Some("boom".to_string()),
        ..FakeSink::default()
    });
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(
        1,
        SenderCommandKind::Prepare {
            patch: LivePatch::default(),
        },
    );
    wait_until(|| harness.status.lock().unwrap().prepare_error().is_some());

    harness.send(2, SenderCommandKind::Stop);
    wait_until(|| harness.status.lock().unwrap().command_id() == 2);

    assert_eq!(harness.status.lock().unwrap().prepare_error(), Some("boom"));
}

/// 持ち越した理由は、それを出した command のものとしてだけ読める。
///
/// 後の command の失敗と取り違えると、鳴った試聴に前の失敗の理由が出る。
#[test]
fn the_reason_belongs_to_the_command_that_failed() {
    let sink = Arc::new(FakeSink {
        prepare_error: Some("boom".to_string()),
        ..FakeSink::default()
    });
    let harness = Harness::spawn(Arc::clone(&sink));
    harness.send(
        1,
        SenderCommandKind::Prepare {
            patch: LivePatch::default(),
        },
    );
    wait_until(|| harness.status.lock().unwrap().prepare_error().is_some());
    harness.send(2, SenderCommandKind::Stop);
    wait_until(|| harness.status.lock().unwrap().command_id() == 2);

    let status = harness.status.lock().unwrap().clone();
    assert_eq!(status.prepare_error_for(1), Some("boom"));
    assert_eq!(status.prepare_error_for(2), None);
}
