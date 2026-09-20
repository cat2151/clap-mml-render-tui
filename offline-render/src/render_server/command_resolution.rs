//! render-server の起動コマンドをどう決めるか。
//!
//! `offline_render_server_command` が明示されていれば shell 経由でそれを使う。
//! 空なら ADR 0017 と同じ探索本体（`cmrt_runtime::resolve_sibling_binary`）で
//! 「同じディレクトリ → 兄弟 repo の release」の順に探す。**PATH は見ない**ので、
//! 見つからなければここで確定的に `Err` になる（呼び出し側が render エラーとして返す）。

use std::{
    path::{Path, PathBuf},
    process::Command,
};

use cmrt_runtime::SiblingBinarySource;

/// render-server の起動コマンドの解決結果。
#[derive(Debug)]
pub(super) enum ResolvedRenderServerCommand {
    /// `offline_render_server_command` が明示された（shell 経由）。
    Shell(String),
    /// 探索（同じディレクトリ → 兄弟 repo の release）で見つかった実行ファイル。
    Binary {
        path: PathBuf,
        source: SiblingBinarySource,
    },
}

impl ResolvedRenderServerCommand {
    /// ログと起動失敗メッセージに出す実体の説明。
    pub(super) fn describe(&self) -> String {
        match self {
            ResolvedRenderServerCommand::Shell(command) => command.clone(),
            ResolvedRenderServerCommand::Binary { path, .. } => path.display().to_string(),
        }
    }

    /// ログの `source=` に出す語。
    pub(super) fn source_label(&self) -> &'static str {
        match self {
            ResolvedRenderServerCommand::Shell(_) => "command",
            ResolvedRenderServerCommand::Binary {
                source: SiblingBinarySource::SiblingDirectory,
                ..
            } => "同じディレクトリ",
            ResolvedRenderServerCommand::Binary {
                source: SiblingBinarySource::SiblingRepoRelease,
                ..
            } => "兄弟 repo の release",
        }
    }

    /// `config_path`（`cmrt --config <path>` の path）のうち、子へ `--config` として
    /// 実際に渡す分。探索で決めた実体にはそのまま渡し、子も同じ config（port・`[plugins]`）
    /// を読む。明示の shell command には渡さない（書かれたとおりに起動する）。
    pub(super) fn forwarded_config_path<'a>(
        &self,
        config_path: Option<&'a Path>,
    ) -> Option<&'a Path> {
        match self {
            ResolvedRenderServerCommand::Shell(_) => None,
            ResolvedRenderServerCommand::Binary { .. } => config_path,
        }
    }

    pub(super) fn build_command(&self, config_path: Option<&Path>) -> Command {
        let mut cmd = match self {
            ResolvedRenderServerCommand::Shell(command) => shell_command(command),
            ResolvedRenderServerCommand::Binary { path, .. } => Command::new(path),
        };
        if let Some(config_path) = self.forwarded_config_path(config_path) {
            cmd.arg("--config").arg(config_path);
        }
        cmd
    }
}

/// `command` が空なら「同じディレクトリ → 兄弟 repo の release」で探す。**PATH は見ない。**
/// 見つからなければ、探した場所を並べたエラー文を返す（呼び出し側は render エラーとして扱う）。
pub(super) fn resolve_render_server_command(
    command: &str,
) -> Result<ResolvedRenderServerCommand, String> {
    resolve_render_server_command_with(command, std::env::current_exe().ok().as_deref())
}

fn resolve_render_server_command_with(
    command: &str,
    current_exe: Option<&Path>,
) -> Result<ResolvedRenderServerCommand, String> {
    let trimmed = command.trim();
    if !trimmed.is_empty() {
        return Ok(ResolvedRenderServerCommand::Shell(trimmed.to_string()));
    }

    cmrt_runtime::resolve_sibling_binary(
        current_exe,
        default_render_server_executable_name(),
        cmrt_runtime::PLAY_SERVER_REPO_DIR_NAME,
    )
    .map(|resolved| ResolvedRenderServerCommand::Binary {
        path: resolved.path,
        source: resolved.source,
    })
    .map_err(|searched| {
        cmrt_runtime::not_found_lines(
            "render-server",
            default_render_server_executable_name(),
            &searched,
        )
        .join("\n")
    })
}

pub(super) fn default_render_server_executable_name() -> &'static str {
    if cfg!(windows) {
        "clap-mml-render-server.exe"
    } else {
        "clap-mml-render-server"
    }
}

#[cfg(target_os = "windows")]
fn shell_command(command: &str) -> Command {
    let mut cmd = Command::new("cmd");
    cmd.arg("/C").arg(command);
    cmd
}

#[cfg(not(target_os = "windows"))]
fn shell_command(command: &str) -> Command {
    let mut cmd = Command::new("sh");
    cmd.arg("-c").arg(command);
    cmd
}

#[cfg(test)]
mod tests;
