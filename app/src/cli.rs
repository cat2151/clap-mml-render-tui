//! コマンドラインの受け取り。**何をするかを決めるだけ**で、実行はしない。
//!
//! 実行は `main.rs` の `run()`。ここが返す [`CliInvocation`] が
//! 「どのモードか」と「全モード共通の指定」の 2 つを持つ。

use anyhow::Result;
use clap::{CommandFactory, FromArgMatches, Parser};
use clap_mml_render_tui::{
    bass_voicing_inspect::BassVoicingInspectRequest,
    dexed_duplicates::DexedDuplicatesRequest,
    guitar_articulation_events::GuitarArticulationEventsRequest,
    inspect_daw_cache::InspectDawCacheRequest,
    live_chord_check::LiveChordCheckRequest,
    live_line_check::{LiveLineCheckRequest, ResidualRequest},
    render_mml::RenderMmlRequest,
    server,
};
use std::path::PathBuf;

use crate::cli_output;

mod commands;
mod keyboard_args;

use commands::Commands;

#[derive(Debug, PartialEq, Eq)]
pub(crate) enum CliAction {
    Help(String),
    Version(String),
    Tui,
    CliMml(String),
    Server(u16),
    Shutdown(u16),
    Update,
    Check,
    ScanLoops,
    BuildVoicingCache {
        force: bool,
    },
    BuildPatchCatalogCache,
    PatchRoles {
        config: Option<PathBuf>,
    },
    DexedDuplicates(DexedDuplicatesRequest),
    RenderMml(RenderMmlRequest),
    InspectDawCache(InspectDawCacheRequest),
    LiveChordCheck(LiveChordCheckRequest),
    LiveLineCheck(LiveLineCheckRequest),
    InspectBassVoicing(BassVoicingInspectRequest),
    GuitarArticulationEvents(GuitarArticulationEventsRequest),
    /// `cmrt kb`。keyboard 画面で起動し、keyboard の状態をこの値へ差し替える。
    Keyboard(cmrt_tui_core::keyboard_session_state::KeyboardSessionState),
}

#[derive(Debug, Parser)]
#[command(
    name = "cmrt",
    about = "CLAP MML Render TUI",
    args_conflicts_with_subcommands = true,
    disable_help_subcommand = true
)]
struct Cli {
    #[arg(
        long,
        num_args = 0..=1,
        value_name = "PORT",
        conflicts_with = "shutdown",
        help = "サーバーモードで起動する"
    )]
    server: Option<Option<u16>>,

    #[arg(
        long,
        num_args = 0..=1,
        value_name = "PORT",
        conflicts_with = "server",
        help = "起動中のサーバーを停止する"
    )]
    shutdown: Option<Option<u16>>,

    /// play server の実体を明示指定する。
    ///
    /// `global = true` なのは、サブコマンド（`render-mml` など）からも play server を
    /// 起こすため。`args_conflicts_with_subcommands` の対象からも外れる。
    #[arg(
        long = "play-server",
        global = true,
        value_name = "PATH",
        help = "play server の実体（フルパス）を指定する"
    )]
    play_server: Option<PathBuf>,

    #[arg(long = "mml", hide = true, num_args = 0..=1, value_name = "MML")]
    deprecated_mml: Option<Option<String>>,

    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(value_name = "MML", help = "CLI モードで再生する MML（テスト用）")]
    mml: Option<String>,
}

fn cli_command() -> clap::Command {
    Cli::command()
        .version(cli_output::version_text())
        .after_help(format!(
            "サーバーモードでは HTTP POST でMMLを受け取りWAVデータを返します。\n  例: curl -X POST http://127.0.0.1:{}/ --data 'cde'",
            server::DEFAULT_PORT
        ))
}

/// 1 回の起動で決まること。`action` に加えて、全モード共通の指定を持つ。
#[derive(Debug, PartialEq, Eq)]
pub(crate) struct CliInvocation {
    pub(crate) action: CliAction,
    /// `--play-server <PATH>`。**探索より強い**（ADR 0017）。
    pub(crate) play_server: Option<PathBuf>,
}

/// `action` だけが要るとき用（テストの読みやすさのため）。
#[cfg(test)]
fn parse_cli_from<I, T>(args: I) -> Result<CliAction>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    parse_cli_invocation_from(args).map(|invocation| invocation.action)
}

pub(crate) fn parse_cli_invocation_from<I, T>(args: I) -> Result<CliInvocation>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = match cli_command().try_get_matches_from_mut(args) {
        Ok(matches) => {
            Cli::from_arg_matches(&matches).map_err(|err| anyhow::anyhow!(err.to_string()))?
        }
        Err(err) if err.kind() == clap::error::ErrorKind::DisplayHelp => {
            return Ok(CliInvocation {
                action: CliAction::Help(err.to_string()),
                play_server: None,
            });
        }
        Err(err) if err.kind() == clap::error::ErrorKind::DisplayVersion => {
            return Ok(CliInvocation {
                action: CliAction::Version(err.to_string()),
                play_server: None,
            });
        }
        Err(err) => return Err(anyhow::anyhow!(err.to_string())),
    };

    if cli.deprecated_mml.is_some() {
        anyhow::bail!(
            "`--mml` オプションは廃止されました。`cmrt <mml>` の形式で指定してください。\n例: cmrt cde"
        );
    }

    // `--play-server` はどのモードでも同じ意味なので、action の分岐とは別に持つ。
    let play_server = cli.play_server.clone();
    let wrap = |action| {
        Ok(CliInvocation {
            action,
            play_server: play_server.clone(),
        })
    };

    if let Some(port) = cli.shutdown {
        let port = port.unwrap_or(server::DEFAULT_PORT);
        return wrap(CliAction::Shutdown(port));
    }

    if let Some(port) = cli.server {
        let port = port.unwrap_or(server::DEFAULT_PORT);
        return wrap(CliAction::Server(port));
    }

    if matches!(cli.command, Some(Commands::Update)) {
        return wrap(CliAction::Update);
    }

    if matches!(cli.command, Some(Commands::Check)) {
        return wrap(CliAction::Check);
    }

    if matches!(cli.command, Some(Commands::ScanLoops)) {
        return wrap(CliAction::ScanLoops);
    }

    if let Some(Commands::PatchRoles { config }) = cli.command {
        return wrap(CliAction::PatchRoles { config });
    }

    if let Some(Commands::DexedDuplicates { config, condition }) = cli.command {
        return wrap(CliAction::DexedDuplicates(DexedDuplicatesRequest {
            config,
            condition: condition.join(" "),
        }));
    }

    if let Some(Commands::RenderMml {
        config,
        patches,
        plugin,
        out_dir,
        poly_check,
        verify,
        mml,
    }) = cli.command
    {
        return wrap(CliAction::RenderMml(RenderMmlRequest {
            config,
            patches,
            plugin,
            mml,
            out_dir,
            poly_check,
            verify,
        }));
    }

    if let Some(Commands::InspectDawCache {
        out_dir,
        delete_broken,
    }) = cli.command
    {
        return wrap(CliAction::InspectDawCache(InspectDawCacheRequest {
            out_dir,
            delete_broken,
        }));
    }

    if let Some(Commands::LiveChordCheck {
        config,
        patch,
        key,
        step_ms,
        out,
        verify,
        degrees,
    }) = cli.command
    {
        return wrap(CliAction::LiveChordCheck(LiveChordCheckRequest {
            config,
            patch,
            key,
            degrees,
            step_ms,
            out,
            verify,
        }));
    }

    if let Some(Commands::LiveLineCheck {
        config,
        step_ms,
        out,
        offline,
        fade_previous_ms,
        residual_reference,
        residual_notch_note,
        lines,
    }) = cli.command
    {
        return wrap(CliAction::LiveLineCheck(LiveLineCheckRequest {
            config,
            lines,
            step_ms,
            out,
            offline,
            fade_previous_ms,
            residual: residual_reference
                .zip(residual_notch_note)
                .map(|(reference, notch_note)| ResidualRequest {
                    reference,
                    notch_note,
                }),
        }));
    }

    if let Some(Commands::InspectBassVoicing { key, progressions }) = cli.command {
        return wrap(CliAction::InspectBassVoicing(BassVoicingInspectRequest {
            key,
            progressions,
        }));
    }

    if let Some(Commands::GuitarArticulationEvents {
        last_played,
        compare_previous,
        rules,
        mml,
    }) = cli.command
    {
        return wrap(CliAction::GuitarArticulationEvents(
            GuitarArticulationEventsRequest {
                last_played,
                compare_previous,
                rules,
                mml,
            },
        ));
    }

    if let Some(Commands::BuildVoicingCache { force }) = cli.command {
        return wrap(CliAction::BuildVoicingCache { force });
    }

    if matches!(cli.command, Some(Commands::BuildPatchCatalogCache)) {
        return wrap(CliAction::BuildPatchCatalogCache);
    }

    if let Some(Commands::Kb(args)) = cli.command {
        return wrap(CliAction::Keyboard(args.into_session_state()));
    }

    if let Some(mml) = cli.mml {
        return wrap(CliAction::CliMml(mml));
    }

    wrap(CliAction::Tui)
}

/// `--play-server <PATH>` を実体の指定へ変える。
///
/// 存在しないパスを黙って探索へ落とさない。打った指定が静かに無視されるのは、
/// ADR 0017 が潰した事故（環境が黙って実体を決める）と同じ手触りになる。
pub(crate) fn play_server_launch(path: PathBuf) -> Result<cmrt_runtime::PlayServerLaunch> {
    if !path.is_file() {
        anyhow::bail!(
            "--play-server で指定された実体がありません: {}",
            path.display()
        );
    }
    Ok(cmrt_runtime::PlayServerLaunch::Executable(path))
}

#[cfg(test)]
mod tests;
