use std::{
    io::{BufRead as _, BufReader},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
    time::Instant,
};

use anyhow::{anyhow, Result};

use super::{
    logging::{log_realtime_play_event, truncate_for_log},
    server_binary::{ResolvedServer, ServerSource},
    startup_failure::StderrCapture,
    RealtimePlayServerStartupPhase, RealtimePlayServerStartupProgress,
};

/// 起動しようとしたコマンドと、その素性。
///
/// `description` はログ 1 行ぶんの key=value 列で、`resolved` は「どの実体を掴んだか」。
/// 落ちたときのエラー文と UI は後者だけを使う。
pub(super) struct ServerLaunch {
    pub(super) command: Command,
    pub(super) description: String,
    pub(super) resolved: ResolvedServer,
}

const STARTUP_PROGRESS_PREFIX: &str = "cmrt-server-startup: instances=";
const STARTUP_PHASE_PREFIX: &str = "cmrt-server-startup: phase=";

pub(super) fn stop_child(child: Option<Child>) {
    let Some(mut child) = child else {
        return;
    };
    if child.try_wait().ok().flatten().is_none() {
        let _ = child.kill();
    }
    let _ = child.wait();
}

pub(super) fn parse_server_startup_progress(line: &str) -> Option<(usize, usize)> {
    let progress = line.strip_prefix(STARTUP_PROGRESS_PREFIX)?;
    let (completed, total) = progress.split_once('/')?;
    let completed = completed.parse().ok()?;
    let total = total.parse().ok()?;
    (total > 0 && completed <= total).then_some((completed, total))
}

pub(super) fn parse_server_startup_phase(line: &str) -> Option<RealtimePlayServerStartupPhase> {
    let phase = line
        .strip_prefix(STARTUP_PHASE_PREFIX)?
        .split_whitespace()
        .next()?;
    match phase {
        "plugin_catalog" => Some(RealtimePlayServerStartupPhase::PluginCatalog),
        "load_entry" => Some(RealtimePlayServerStartupPhase::LoadEntry),
        "instances" => Some(RealtimePlayServerStartupPhase::Instances),
        "audio_stream" => Some(RealtimePlayServerStartupPhase::AudioStream),
        "listen" => Some(RealtimePlayServerStartupPhase::Listen),
        _ => None,
    }
}

fn apply_server_startup_line(progress: &mut RealtimePlayServerStartupProgress, line: &str) {
    if let Some(phase) = parse_server_startup_phase(line) {
        progress.advance_to(phase);
    }
    if let Some((initialized_instances, total_instances)) = parse_server_startup_progress(line) {
        progress.advance_to(RealtimePlayServerStartupPhase::Instances);
        progress.initialized_instances = initialized_instances;
        progress.total_instances = total_instances;
    }
}

fn server_spawned_log_line(
    port: u16,
    pid: u32,
    launch_description: &str,
    spawn_ms: u128,
) -> String {
    format!(
        "action=server-spawned phase=server_exe_spawn ms={spawn_ms} port={port} pid={pid} {launch_description}"
    )
}

/// 決まった実体から、実際に spawn するコマンドを組み立てる。
///
/// **どこを探すかは [`crate::server_binary`] の仕事**で、ここはその結果を
/// `Command` にするだけ。PATH は見ない（ADR 0017）。
pub(super) fn build_realtime_play_server_command(resolved: &ResolvedServer) -> ServerLaunch {
    let command = match resolved.source {
        // テストの偽サーバーだけが shell を通る。`echo ... & exit 3` のような
        // 「即死するサーバー」は shell が無いと書けない。
        ServerSource::ShellCommand => shell_command(&resolved.exe),
        _ => Command::new(&resolved.exe),
    };
    ServerLaunch {
        command,
        description: resolved.log_fields(),
        resolved: resolved.clone(),
    }
}

pub(super) fn spawn_realtime_play_server(
    mut command: Command,
    launch_description: &str,
    port: u16,
    live_instance_count: usize,
    startup_progress: Arc<Mutex<Option<RealtimePlayServerStartupProgress>>>,
    stderr_capture: StderrCapture,
) -> Result<Child> {
    *startup_progress.lock().unwrap() = Some(RealtimePlayServerStartupProgress::starting(
        live_instance_count,
    ));
    log_realtime_play_event(format!(
        "action=server-spawn port={port} {launch_description}"
    ));
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let spawn_started = Instant::now();
    let mut child = command.spawn().map_err(|error| {
        anyhow!("realtime play server の起動に失敗しました ({launch_description}): {error}")
    })?;
    let spawn_ms = spawn_started.elapsed().as_millis();
    let pid = child.id();
    if let Some(progress) = startup_progress.lock().unwrap().as_mut() {
        progress.server_exe_spawned = true;
    }
    // 子の stderr reader より先に残し、exe 起動完了と子プロセス内フェーズの順序を
    // log.txt 上でも保証する。
    log_realtime_play_event(server_spawned_log_line(
        port,
        pid,
        launch_description,
        spawn_ms,
    ));
    if let Some(stderr) = child.stderr.take() {
        let thread_progress = Arc::clone(&startup_progress);
        let thread_capture = stderr_capture.clone();
        let thread_result = std::thread::Builder::new()
            .name("realtime-play-server-stderr".to_string())
            .spawn(move || {
                for line in BufReader::new(stderr).lines() {
                    match line {
                        Ok(line) => {
                            if let Some(progress) = thread_progress.lock().unwrap().as_mut() {
                                apply_server_startup_line(progress, &line);
                            }
                            log_realtime_play_event(format!(
                                "action=server-stderr pid={pid} line=\"{}\"",
                                truncate_for_log(&line, 1_000)
                            ));
                            // 落ちたときに「なぜ」を言えるよう、末尾だけ手元に残す。
                            thread_capture.push(truncate_for_log(&line, 1_000));
                        }
                        Err(error) => {
                            log_realtime_play_event(format!(
                                "action=server-stderr-read-error pid={pid} error={error:?}"
                            ));
                            break;
                        }
                    }
                }
                thread_capture.mark_finished();
            });
        if let Err(error) = thread_result {
            log_realtime_play_event(format!(
                "action=server-stderr-reader-start-error pid={pid} error={error:?}"
            ));
            // 読み手が居ない以上、待っても stderr は 1 行も増えない。
            stderr_capture.mark_finished();
        }
    } else {
        stderr_capture.mark_finished();
    }
    Ok(child)
}

#[cfg(target_os = "windows")]
pub(super) fn shell_command(command: &str) -> Command {
    let mut cmd = Command::new("cmd");
    cmd.arg("/C").arg(command);
    cmd
}

#[cfg(test)]
mod tests;

#[cfg(not(target_os = "windows"))]
pub(super) fn shell_command(command: &str) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);
    cmd
}
