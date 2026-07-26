use std::process::{Child, Command};

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
