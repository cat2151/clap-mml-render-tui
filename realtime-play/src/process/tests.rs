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

#[test]
fn startup_phase_parser_accepts_the_server_protocol_and_ignores_unknown_phases() {
    assert_eq!(
        parse_server_startup_phase(
            "cmrt-server-startup: phase=plugin_catalog event=begin since_boot_ms=37"
        ),
        Some(RealtimePlayServerStartupPhase::PluginCatalog)
    );
    assert_eq!(
        parse_server_startup_phase("cmrt-server-startup: phase=load_entry event=begin"),
        Some(RealtimePlayServerStartupPhase::LoadEntry)
    );
    assert_eq!(
        parse_server_startup_phase("cmrt-server-startup: phase=instances event=begin"),
        Some(RealtimePlayServerStartupPhase::Instances)
    );
    assert_eq!(
        parse_server_startup_phase("cmrt-server-startup: phase=audio_stream event=begin"),
        Some(RealtimePlayServerStartupPhase::AudioStream)
    );
    assert_eq!(
        parse_server_startup_phase("cmrt-server-startup: phase=listen event=begin"),
        Some(RealtimePlayServerStartupPhase::Listen)
    );
    assert_eq!(
        parse_server_startup_phase("cmrt-server-startup: phase=future event=begin"),
        None
    );
}

#[test]
fn startup_lines_advance_one_shared_snapshot_without_erasing_spawn_state() {
    let mut progress = RealtimePlayServerStartupProgress::starting(14);
    progress.server_exe_spawned = true;

    apply_server_startup_line(
        &mut progress,
        "cmrt-server-startup: phase=plugin_catalog event=begin since_boot_ms=3",
    );
    assert_eq!(
        progress.phase,
        Some(RealtimePlayServerStartupPhase::PluginCatalog)
    );
    assert!(progress.server_exe_spawned);

    apply_server_startup_line(&mut progress, "cmrt-server-startup: instances=5/14");
    assert_eq!(
        progress.phase,
        Some(RealtimePlayServerStartupPhase::Instances)
    );
    assert_eq!(progress.initialized_instances, 5);
    assert_eq!(progress.total_instances, 14);
    assert!(progress.server_exe_spawned);
}

#[test]
fn spawned_log_identifies_the_exe_boundary_and_its_elapsed_time() {
    let port = 42_154;
    assert_eq!(
        server_spawned_log_line(port, 4321, "source=sibling", 287),
        format!(
            "action=server-spawned phase=server_exe_spawn ms=287 port={port} pid=4321 source=sibling"
        )
    );
}
