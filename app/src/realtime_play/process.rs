use std::process::{Child, Command};

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

#[cfg(not(target_os = "windows"))]
pub(super) fn shell_command(command: &str) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);
    cmd
}
