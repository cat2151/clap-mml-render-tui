use super::*;

use crate::server_binary::ServerProfile;

/// 実体そのものを起動する経路は shell を通さない。
/// スペースを含むパスが shell の語分割で壊れるのを防ぐ。
#[test]
fn a_resolved_executable_is_spawned_directly() {
    let resolved = ResolvedServer {
        exe: r"C:\Program Files\cmrt\clap-mml-realtime-play-server.exe".to_owned(),
        source: ServerSource::SiblingDirectory,
        profile: ServerProfile::Bundled,
        stale: None,
    };

    let launch = build_realtime_play_server_command(&resolved);

    assert_eq!(
        launch.command.get_program(),
        std::ffi::OsStr::new(&resolved.exe)
    );
    assert_eq!(launch.command.get_args().count(), 0);
}

/// ログ 1 行に「どれで決まったか」「どういう素性か」「どの実体か」が全部載る。
/// 症状から原因へ辿るとき、最初に要るのがこの 3 つだった。
#[test]
fn the_log_line_carries_source_profile_and_fullpath() {
    let resolved = ResolvedServer {
        exe: "/x/clap-mml-play-server/target/release/clap-mml-realtime-play-server".to_owned(),
        source: ServerSource::PlayServerRepoRelease,
        profile: ServerProfile::Release,
        stale: None,
    };

    let description = build_realtime_play_server_command(&resolved).description;

    assert!(
        description.contains("source=play-server-repo-release"),
        "{description}"
    );
    assert!(description.contains("profile=release"), "{description}");
    assert!(description.contains(&resolved.exe), "{description}");
}

#[test]
fn startup_progress_parser_accepts_only_valid_instance_counts() {
    assert_eq!(
        parse_server_startup_progress("cmrt-server-startup: instances=7/16"),
        Some((7, 16))
    );
    assert_eq!(
        parse_server_startup_progress("cmrt-server-startup: instances=17/16"),
        None
    );
    assert_eq!(parse_server_startup_progress("unrelated output"), None);
}
