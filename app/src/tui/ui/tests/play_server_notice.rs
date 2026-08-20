//! play server が起動できないときに、その理由が実際に画面へ出ることの確認。
//!
//! 「無音になるだけで理由がどこにも出ない」のが直したかった状態なので、
//! ここは文字列の組み立てではなく、描画された画面そのものを見る。

use std::net::TcpListener;

use super::{render_lines, test_config};
use crate::tui::TuiApp;

/// stderr へ 1 行残してから失敗するコマンド。shell 経由で起動される前提。
fn immediately_failing_command() -> &'static str {
    if cfg!(windows) {
        "echo Error: boom 1>&2& exit 3"
    } else {
        "echo 'Error: boom' 1>&2; exit 3"
    }
}

/// 全角文字は 1 文字でセルを 2 つ使うので、TestBackend の行文字列では空白が挟まる。
/// 既存の `find_text_ignoring_spaces` と同じく、空白を落としてから照合する。
fn contains_ignoring_spaces(lines: &[String], text: &str) -> bool {
    let strip = |value: &str| -> String { value.chars().filter(|c| !c.is_whitespace()).collect() };
    let needle = strip(text);
    lines.iter().any(|line| strip(line).contains(&needle))
}

fn app_with_a_failing_play_server() -> TuiApp<'static> {
    let mut cfg = test_config();
    // 誰も listen していないポート。開いていると「起動できた」ことになってしまう。
    cfg.realtime_play_server_port = TcpListener::bind("127.0.0.1:0")
        .unwrap()
        .local_addr()
        .unwrap()
        .port();
    cfg.realtime_play_server_command = immediately_failing_command().to_string();

    let app = TuiApp::new_for_test(cfg);
    // 実際に起動を試みて失敗させる。理由はここで supervisor に記録される。
    app.play_server.ensure_started_for_fast_midi().unwrap_err();
    app
}

#[test]
fn the_screen_shows_why_the_play_server_could_not_start() {
    let mut app = app_with_a_failing_play_server();

    let lines = render_lines(&mut app, 120, 30);

    assert!(
        contains_ignoring_spaces(&lines, "play server が起動できません"),
        "起動できないことを画面に出すこと: {lines:?}"
    );
    assert!(
        contains_ignoring_spaces(&lines, immediately_failing_command()),
        "どの実体を起動したのかを画面に出すこと: {lines:?}"
    );
    assert!(
        contains_ignoring_spaces(&lines, "Error: boom"),
        "子が言い残した理由を画面に出すこと: {lines:?}"
    );
}

/// 出しっぱなしにはしない。閉じたら同じ理由では出し直さない。
#[test]
fn the_notice_goes_away_once_it_is_dismissed() {
    let mut app = app_with_a_failing_play_server();
    assert!(app.dismiss_play_server_notice());

    let lines = render_lines(&mut app, 120, 30);

    assert!(
        !contains_ignoring_spaces(&lines, "play server が起動できません"),
        "閉じたあとは出さないこと: {lines:?}"
    );
    assert!(
        !app.dismiss_play_server_notice(),
        "閉じるものが無いときはキーを画面側へ通すこと"
    );
}
