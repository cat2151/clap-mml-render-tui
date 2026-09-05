use cmrt_realtime_play::{ServerProfile, ServerSource, StaleSource};
use ratatui::{backend::TestBackend, Terminal};

use super::*;

fn resolved(exe: &str, profile: ServerProfile) -> ServerBinary {
    ServerBinary::Resolved(ResolvedServer {
        exe: exe.to_owned(),
        source: ServerSource::PlayServerRepoRelease,
        profile,
        stale: None,
    })
}

/// 素性は通常運転（release）だが、実体がソースより古いもの。
fn stale_release(exe: &str) -> ServerBinary {
    let ServerBinary::Resolved(mut resolved) = resolved(exe, ServerProfile::Release) else {
        unreachable!()
    };
    resolved.stale = Some(StaleSource {
        newest_source: "core-lib/src/lib.rs".to_owned(),
        newer_by_seconds: 42,
    });
    ServerBinary::Resolved(resolved)
}

/// 画面 1 枚ぶん描いて、そこに出た文字を全部つなげたもの。
///
/// 全角文字は 1 文字でセルを 2 つ使い、後ろのセルが空白として並ぶ。
/// 照合は空白を落としてから行うこと。
fn rendered(binary: &ServerBinary, width: u16) -> String {
    let mut terminal = Terminal::new(TestBackend::new(width, 8)).unwrap();
    terminal.draw(|frame| draw(frame, binary)).unwrap();
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .flat_map(|cell| cell.symbol().chars())
        .filter(|c| !c.is_whitespace())
        .collect()
}

/// debug のサーバーを掴んでいるときだけ画面の文字列が変わる、というのがこの機能の全部。
#[test]
fn a_debug_server_changes_what_is_on_screen() {
    let debug = resolved(
        r"N:\projects\clap-mml-play-server\target\debug\clap-mml-realtime-play-server.exe",
        ServerProfile::Debug,
    );
    let release = resolved(
        r"N:\projects\clap-mml-play-server\target\release\clap-mml-realtime-play-server.exe",
        ServerProfile::Release,
    );

    let with_debug = rendered(&debug, 80);

    assert!(with_debug.contains("playserver:debug"), "{with_debug:?}");
    assert!(
        with_debug.contains("clap-mml-realtime-play-server.exe"),
        "どの実体を掴んだかも出すこと: {with_debug:?}"
    );
    assert!(
        !rendered(&release, 80).contains("playserver"),
        "release は通常運転なので静かでよい"
    );
}

#[test]
fn a_bundled_server_is_silent_but_an_unknown_one_is_not() {
    let bundled = resolved(
        r"C:\Users\x\cmrt\clap-mml-realtime-play-server.exe",
        ServerProfile::Bundled,
    );
    let unknown = resolved("exit 0", ServerProfile::Unknown);

    assert!(!rendered(&bundled, 80).contains("playserver"));
    assert!(rendered(&unknown, 80).contains("playserver:不明"));
}

#[test]
fn a_server_that_could_not_be_found_is_left_to_the_startup_failure_notice() {
    let not_found = ServerBinary::NotFound {
        searched: vec!["どこか".to_owned()],
    };

    assert!(!rendered(&not_found, 80).contains("playserver"));
}

/// 右端に寄せる。左端から書くと、どの画面でも中身の 1 行目を潰す。
#[test]
fn the_badge_sits_at_the_top_right() {
    let text = "⚠ play server: debug [x]";
    let area = badge_area(Rect::new(0, 0, 80, 24), text).unwrap();

    assert_eq!(area.y, 0);
    assert_eq!(area.height, 1);
    assert_eq!(
        area.x + area.width,
        80 - RIGHT_MARGIN,
        "右端から 1 桁空けて終わること"
    );
}

/// 入らないなら出さない。無理に描くと画面が壊れて、本題の演奏より困る。
#[test]
fn a_narrow_terminal_gets_no_badge() {
    assert_eq!(
        badge_area(Rect::new(0, 0, 10, 24), "⚠ play server: debug"),
        None
    );
    assert_eq!(
        badge_area(Rect::new(0, 0, 80, 0), "⚠ play server: debug"),
        None
    );
}

#[test]
fn the_exe_tail_keeps_the_profile_directory_in_both_spellings() {
    assert_eq!(
        exe_tail(r"N:\a\b\target\debug\clap-mml-realtime-play-server.exe"),
        "debug/clap-mml-realtime-play-server.exe"
    );
    assert_eq!(
        exe_tail("/home/x/target/debug/clap-mml-realtime-play-server"),
        "debug/clap-mml-realtime-play-server"
    );
    // 区切りが無いもの（テストの偽サーバー）はそのまま。
    assert_eq!(exe_tail("exit 0"), "exit 0");
}

/// 素性が通常運転（release）でも、実体がソースより古ければ点灯する。
/// PATH 解決を潰した代わりに生まれた穴がここで見える。
#[test]
fn a_release_binary_older_than_its_sources_still_lights_up() {
    let stale = stale_release(
        r"N:\projects\clap-mml-play-server\target\release\clap-mml-realtime-play-server.exe",
    );

    let out = rendered(&stale, 80);

    assert!(out.contains("playserver:release"), "{out:?}");
    assert!(
        out.contains("ソースより古い"),
        "なぜ点いたのかを言うこと: {out:?}"
    );
}

/// 狭い端末では実体のパスを落とす。警告そのものが消えるのが一番まずい。
#[test]
fn a_narrow_terminal_keeps_the_warning_and_drops_the_path() {
    let debug = resolved(
        r"N:\projects\clap-mml-play-server\target\debug\clap-mml-realtime-play-server.exe",
        ServerProfile::Debug,
    );

    let out = rendered(&debug, 30);

    assert!(out.contains("playserver:debug"), "{out:?}");
    assert!(
        !out.contains("clap-mml-realtime-play-server.exe"),
        "入らないパスは落とすこと: {out:?}"
    );
}
