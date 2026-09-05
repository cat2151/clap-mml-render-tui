use std::{
    io::{BufRead as _, BufReader},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, Result};

use super::{
    logging::{log_realtime_play_event, truncate_for_log},
    server_binary::{ResolvedServer, ServerSource},
    startup_failure::StderrCapture,
    RealtimePlayServerStartupProgress,
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
    *startup_progress.lock().unwrap() = Some(RealtimePlayServerStartupProgress {
        initialized_instances: 0,
        total_instances: live_instance_count,
    });
    log_realtime_play_event(format!(
        "action=server-spawn port={port} {launch_description}"
    ));
    command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::piped());
    let mut child = command.spawn().map_err(|error| {
        anyhow!("realtime play server の起動に失敗しました ({launch_description}): {error}")
    })?;
    let pid = child.id();
    if let Some(stderr) = child.stderr.take() {
        let thread_progress = Arc::clone(&startup_progress);
        let thread_capture = stderr_capture.clone();
        let thread_result = std::thread::Builder::new()
            .name("realtime-play-server-stderr".to_string())
            .spawn(move || {
                for line in BufReader::new(stderr).lines() {
                    match line {
                        Ok(line) => {
                            if let Some((initialized_instances, total_instances)) =
                                parse_server_startup_progress(&line)
                            {
                                *thread_progress.lock().unwrap() =
                                    Some(RealtimePlayServerStartupProgress {
                                        initialized_instances,
                                        total_instances,
                                    });
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
    log_realtime_play_event(format!(
        "action=server-spawned port={port} pid={pid} {launch_description}"
    ));
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
