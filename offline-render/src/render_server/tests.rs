use std::net::TcpListener;

use super::*;

mod parallel_effect_chain;

fn supervisor_for_listening_port(listener: &TcpListener) -> RenderServerSupervisor {
    let port = listener.local_addr().unwrap().port();
    let cfg: Config = toml::from_str(&format!(
        r#"
plugin_path = "dummy.clap"
input_midi = "input.mid"
output_midi = "output.mid"
output_wav = "output.wav"
sample_rate = 48000
buffer_size = 512
offline_render_server_port = {port}
offline_render_server_command = "exit 0"
"#
    ))
    .unwrap();
    RenderServerSupervisor::new(&cfg)
}

#[test]
fn stale_transport_failure_reuses_newer_generation_without_restart() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let supervisor = supervisor_for_listening_port(&listener);
    supervisor.set_generation_for_test(2);

    let generation = supervisor.recover_after_transport_failure(1).unwrap();

    assert_eq!(generation, 2);
    assert_eq!(supervisor.spawn_count_for_test(), 0);
}

#[test]
fn current_generation_transport_failure_restarts_server() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let supervisor = supervisor_for_listening_port(&listener);
    supervisor.set_generation_for_test(7);

    let generation = supervisor.recover_after_transport_failure(7).unwrap();

    assert!(generation > 7);
    assert_eq!(supervisor.restart_count_for_test(), 1);
    assert_eq!(supervisor.spawn_count_for_test(), 1);
}

#[test]
fn never_kill_mode_fails_the_render_instead_of_restarting_the_server() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut supervisor = supervisor_for_listening_port(&listener);
    supervisor.set_never_kill_for_test(true);
    supervisor.set_generation_for_test(7);

    let error = supervisor.recover_after_transport_failure(7).unwrap_err();

    assert!(error.to_string().contains(NEVER_KILL_ENV), "{error:#}");
    assert_eq!(supervisor.restart_count_for_test(), 0);
    assert_eq!(supervisor.spawn_count_for_test(), 0);
}

#[test]
fn never_kill_mode_still_reuses_a_newer_generation() {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let mut supervisor = supervisor_for_listening_port(&listener);
    supervisor.set_never_kill_for_test(true);
    supervisor.set_generation_for_test(2);

    let generation = supervisor.recover_after_transport_failure(1).unwrap();

    assert_eq!(generation, 2);
    assert_eq!(supervisor.restart_count_for_test(), 0);
}

/// `config=` は子へ実際に渡したときだけ出す。shell command のときに出すと、
/// 子が読んでいない config を読んだように見える。
#[test]
fn the_resolved_log_line_names_the_config_path_only_when_it_is_forwarded() {
    let binary = ResolvedRenderServerCommand::Binary {
        path: PathBuf::from("render-server.exe"),
        source: cmrt_runtime::SiblingBinarySource::SiblingDirectory,
    };
    let shell = ResolvedRenderServerCommand::Shell("exit 0".to_string());
    let config_path = Path::new("alt-config.toml");

    assert_eq!(
        render_server_resolved_log_message(&binary, Some(config_path)),
        "render-server: exe=render-server.exe (source=同じディレクトリ) config=alt-config.toml"
    );
    assert_eq!(
        render_server_resolved_log_message(&binary, None),
        "render-server: exe=render-server.exe (source=同じディレクトリ)"
    );
    assert_eq!(
        render_server_resolved_log_message(&shell, Some(config_path)),
        "render-server: exe=exit 0 (source=command)"
    );
}

#[test]
fn render_server_stderr_is_formatted_for_the_app_log() {
    assert_eq!(
        render_server_stderr_log_message(42, "11 helper files excluded"),
        "backend=render_server event=server-stderr pid=42 line=\"11 helper files excluded\""
    );
}

fn run_and_wait(command: &str) -> std::process::ExitStatus {
    #[cfg(target_os = "windows")]
    let status = std::process::Command::new("cmd")
        .arg("/C")
        .arg(command)
        .status();
    #[cfg(not(target_os = "windows"))]
    let status = std::process::Command::new("sh")
        .arg("-c")
        .arg(command)
        .status();
    status.unwrap()
}

#[test]
fn exit_status_log_fields_reports_a_clean_exit() {
    let status = run_and_wait("exit 0");

    assert_eq!(
        exit_status_log_fields(&status),
        "success=true code=0 code_hex=0x00000000"
    );
}

#[test]
fn exit_status_log_fields_reports_a_nonzero_exit_code() {
    let status = run_and_wait("exit 3");

    let fields = exit_status_log_fields(&status);
    assert!(fields.contains("success=false"));
    assert!(fields.contains("code=3"));
    assert!(fields.contains("code_hex=0x00000003"));
}
