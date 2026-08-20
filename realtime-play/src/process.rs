use std::{
    io::{BufRead as _, BufReader},
    process::{Child, Command, Stdio},
    sync::{Arc, Mutex},
};

use anyhow::{anyhow, Result};

use super::{
    logging::{log_realtime_play_event, truncate_for_log},
    startup_failure::StderrCapture,
    RealtimePlayServerStartupProgress,
};

/// 起動しようとしたコマンドと、その素性。
///
/// `description` はログ 1 行ぶんの key=value 列で、`exe` は「どの実体を掴んだか」だけを
/// 取り出したもの。落ちたときのエラー文と UI は後者だけを使う。
pub(super) struct ServerLaunch {
    pub(super) command: Command,
    pub(super) description: String,
    pub(super) exe: String,
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

pub(super) fn sibling_realtime_play_server_path() -> Option<std::path::PathBuf> {
    let current_exe = std::env::current_exe().ok()?;
    let sibling = current_exe
        .parent()?
        .join(default_realtime_play_server_executable_name());
    sibling.is_file().then_some(sibling)
}

pub(super) fn path_realtime_play_server_path() -> Option<std::path::PathBuf> {
    let executable = default_realtime_play_server_executable_name();
    executable_in_paths(
        executable,
        std::env::split_paths(&std::env::var_os("PATH")?),
    )
}

fn executable_in_paths(
    executable: &str,
    mut paths: impl Iterator<Item = std::path::PathBuf>,
) -> Option<std::path::PathBuf> {
    paths.find_map(|directory| {
        let candidate = directory.join(executable);
        if !candidate.is_file() {
            return None;
        }
        if candidate.is_absolute() {
            Some(candidate)
        } else {
            std::env::current_dir()
                .ok()
                .map(|current_dir| current_dir.join(candidate))
        }
    })
}

pub(super) fn parse_server_startup_progress(line: &str) -> Option<(usize, usize)> {
    let progress = line.strip_prefix(STARTUP_PROGRESS_PREFIX)?;
    let (completed, total) = progress.split_once('/')?;
    let completed = completed.parse().ok()?;
    let total = total.parse().ok()?;
    (total > 0 && completed <= total).then_some((completed, total))
}

pub(super) fn default_realtime_play_server_executable_name() -> &'static str {
    if cfg!(windows) {
        "clap-mml-realtime-play-server.exe"
    } else {
        "clap-mml-realtime-play-server"
    }
}

pub(super) fn build_realtime_play_server_command(configured: &str) -> ServerLaunch {
    let trimmed = configured.trim();
    if !trimmed.is_empty() {
        return ServerLaunch {
            command: shell_command(trimmed),
            description: format!("source=config shell_command={trimmed:?}"),
            exe: trimmed.to_owned(),
        };
    }

    if let Some(path) = sibling_realtime_play_server_path() {
        return ServerLaunch {
            command: Command::new(&path),
            description: format!("source=sibling fullpath=\"{}\"", path.display()),
            exe: path.display().to_string(),
        };
    }
    if let Some(path) = path_realtime_play_server_path() {
        return ServerLaunch {
            command: Command::new(&path),
            description: format!("source=PATH fullpath=\"{}\"", path.display()),
            exe: path.display().to_string(),
        };
    }
    let executable = default_realtime_play_server_executable_name();
    ServerLaunch {
        command: Command::new(executable),
        description: format!("source=unresolved-PATH executable={executable:?}"),
        exe: executable.to_owned(),
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
