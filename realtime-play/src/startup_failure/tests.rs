use std::time::{Duration, Instant};

use super::{
    ExitLatch, ServerStartupFailure, StderrCapture, EXIT_LATCH_RESET, MAX_CONSECUTIVE_EXITS,
    STDERR_TAIL_LINES,
};

fn failure(stderr_tail: &[&str]) -> ServerStartupFailure {
    ServerStartupFailure {
        exe: r"C:\bin\clap-mml-realtime-play-server.exe".to_owned(),
        exit_code: Some(1),
        stderr_tail: stderr_tail.iter().map(|line| (*line).to_owned()).collect(),
    }
}

/// この種の事故で最初に要るのは「掴んだ実体」と「子が言い残したこと」の 2 つ。
#[test]
fn message_carries_the_exe_and_the_reason_the_child_left_behind() {
    assert_eq!(
        failure(&["Error: config.toml のプラグイン設定が不正"]).message(),
        "realtime play server が起動できません \
         (exe=\"C:\\bin\\clap-mml-realtime-play-server.exe\", exit=1): \
         Error: config.toml のプラグイン設定が不正"
    );
}

#[test]
fn message_joins_multiple_stderr_lines_into_one_line() {
    assert_eq!(
        failure(&["first", "second"]).message(),
        "realtime play server が起動できません \
         (exe=\"C:\\bin\\clap-mml-realtime-play-server.exe\", exit=1): first / second"
    );
}

/// stderr が空でも「理由が残っていない」ことは言う。無言で消えるのが今回の問題だった。
#[test]
fn message_says_so_when_the_child_left_nothing_on_stderr() {
    let mut failure = failure(&[]);
    failure.exit_code = None;
    assert_eq!(
        failure.message(),
        "realtime play server が起動できません \
         (exe=\"C:\\bin\\clap-mml-realtime-play-server.exe\", exit=不明): (stderr に出力なし)"
    );
}

/// 狭い端末で下が切れても切り分けられるよう、exe は先頭付近に置く。
#[test]
fn lines_put_the_exe_before_the_stderr_tail() {
    assert_eq!(
        failure(&["Error: boom"]).lines(),
        vec![
            "play server が起動できません".to_owned(),
            "exe=\"C:\\bin\\clap-mml-realtime-play-server.exe\"".to_owned(),
            "exit=1".to_owned(),
            "Error: boom".to_owned(),
        ]
    );
}

#[test]
fn stderr_capture_keeps_only_the_last_lines() {
    let capture = StderrCapture::default();
    for index in 0..(STDERR_TAIL_LINES + 3) {
        capture.push(format!("line{index}"));
    }
    capture.mark_finished();

    let snapshot = capture.drain_snapshot();
    assert_eq!(snapshot.len(), STDERR_TAIL_LINES);
    assert_eq!(snapshot.first().unwrap(), "line3");
    assert_eq!(
        snapshot.last().unwrap(),
        &format!("line{}", STDERR_TAIL_LINES + 2)
    );
}

/// 孫プロセスが stderr を握ったまま残っても、待ち続けずに持っているぶんを返す。
#[test]
fn stderr_capture_returns_what_it_has_when_the_reader_never_finishes() {
    let capture = StderrCapture::default();
    capture.push("Error: boom".to_owned());

    assert_eq!(
        capture.drain_snapshot_within(Duration::from_millis(20)),
        vec!["Error: boom".to_owned()]
    );
}

#[test]
fn exit_latch_stops_spawning_after_consecutive_exits() {
    let now = Instant::now();
    let mut latch = ExitLatch::default();

    for _ in 0..MAX_CONSECUTIVE_EXITS {
        assert!(!latch.engaged(now), "打ち切る前は spawn を許す");
        latch.record_exit(now);
    }

    assert!(latch.engaged(now));
}

/// 起動できたら数え直す。長く動いたサーバーが後で落ちたときに、
/// 昔の失敗を引きずって再起動できなくならないようにするため。
#[test]
fn exit_latch_forgets_the_count_once_the_server_starts() {
    let now = Instant::now();
    let mut latch = ExitLatch::default();
    for _ in 0..MAX_CONSECUTIVE_EXITS {
        latch.record_exit(now);
    }

    latch.reset();

    assert!(!latch.engaged(now));
}

/// 間が空いたら、ユーザーが config を直したかもしれないのでもう一度試す。
#[test]
fn exit_latch_releases_after_the_reset_interval() {
    let now = Instant::now();
    let mut latch = ExitLatch::default();
    for _ in 0..MAX_CONSECUTIVE_EXITS {
        latch.record_exit(now);
    }
    assert!(latch.engaged(now));

    assert!(!latch.engaged(now + EXIT_LATCH_RESET));
}
