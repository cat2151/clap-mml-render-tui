use cmrt_realtime_play::ServerStartupFailure;

use super::{notice_lines, notice_width, wrap_to_width, DISMISS_HINT, MAX_NOTICE_WIDTH};

fn failure() -> ServerStartupFailure {
    ServerStartupFailure {
        exe: r"C:\bin\clap-mml-realtime-play-server.exe".to_owned(),
        exit_code: Some(1),
        stderr_tail: vec!["Error: config.toml のプラグイン設定が不正".to_owned()],
    }
}

/// 掴んだ実体と子の言い分を、閉じ方の案内と一緒に出す。
#[test]
fn notice_shows_the_exe_the_reason_and_how_to_close_it() {
    let lines: Vec<String> = notice_lines(&failure(), 80)
        .into_iter()
        .map(|line| line.to_string())
        .collect();

    assert_eq!(lines.first().unwrap(), "play server が起動できません");
    assert!(
        lines
            .iter()
            .any(|line| line.contains("clap-mml-realtime-play-server.exe")),
        "掴んだ実体を出すこと: {lines:?}"
    );
    assert!(
        lines
            .iter()
            .any(|line| line.contains("config.toml のプラグイン設定が不正")),
        "子が言い残した理由を出すこと: {lines:?}"
    );
    assert_eq!(lines.last().unwrap(), DISMISS_HINT);
}

/// 端末幅からはみ出さない。長い exe パスと長いエラー文が両方来るので必須。
#[test]
fn notice_lines_never_exceed_the_given_width() {
    for line in notice_lines(&failure(), 24) {
        assert!(line.width() <= 24, "はみ出した行: {line:?}");
    }
}

#[test]
fn wrap_counts_display_columns_not_characters() {
    // 日本語 1 文字は 2 桁ぶん。4 桁なら 2 文字ずつに割れる。
    assert_eq!(wrap_to_width("あいうえ", 4), vec!["あい", "うえ"]);
    assert_eq!(wrap_to_width("abcdef", 4), vec!["abcd", "ef"]);
}

#[test]
fn wrap_keeps_a_short_line_as_it_is() {
    assert_eq!(wrap_to_width("short", 40), vec!["short"]);
    assert_eq!(wrap_to_width("", 40), vec![""]);
}

/// 横に広い端末でも、読める幅で止める。
#[test]
fn notice_width_is_capped_and_leaves_room_for_the_frame() {
    assert_eq!(notice_width(40), 34);
    assert_eq!(notice_width(1_000), MAX_NOTICE_WIDTH);
    assert_eq!(notice_width(2), 0);
}
