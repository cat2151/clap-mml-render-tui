use anyhow::{Context, Result};
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use clap_mml_render_tui::{config, server, tui, updater};
use cmrt_core::{load_entry, mml_to_play};

#[derive(Debug, PartialEq, Eq)]
enum CliAction {
    Help(String),
    Tui,
    CliMml(String),
    Server(u16),
    Shutdown(u16),
    Update,
    Check,
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

    #[arg(long = "mml", hide = true, num_args = 0..=1, value_name = "MML")]
    deprecated_mml: Option<Option<String>>,

    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(value_name = "MML", help = "CLI モードで再生する MML（テスト用）")]
    mml: Option<String>,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// アップデートを実行
    Update,
    /// ビルド時コミットと remote main を比較
    Check,
}

fn cli_command() -> clap::Command {
    Cli::command().after_help(format!(
        "サーバーモードでは HTTP POST でMMLを受け取りWAVデータを返します。\n  例: curl -X POST http://127.0.0.1:{}/ --data 'cde'",
        server::DEFAULT_PORT
    ))
}

fn parse_cli_from<I, T>(args: I) -> Result<CliAction>
where
    I: IntoIterator<Item = T>,
    T: Into<std::ffi::OsString> + Clone,
{
    let cli = match cli_command().try_get_matches_from_mut(args) {
        Ok(matches) => {
            Cli::from_arg_matches(&matches).map_err(|err| anyhow::anyhow!(err.to_string()))?
        }
        Err(err) if err.kind() == clap::error::ErrorKind::DisplayHelp => {
            return Ok(CliAction::Help(err.to_string()));
        }
        Err(err) => return Err(anyhow::anyhow!(err.to_string())),
    };

    if cli.deprecated_mml.is_some() {
        anyhow::bail!(
            "`--mml` オプションは廃止されました。`cmrt <mml>` の形式で指定してください。\n例: cmrt cde"
        );
    }

    if let Some(port) = cli.shutdown {
        let port = port.unwrap_or(server::DEFAULT_PORT);
        return Ok(CliAction::Shutdown(port));
    }

    if let Some(port) = cli.server {
        let port = port.unwrap_or(server::DEFAULT_PORT);
        return Ok(CliAction::Server(port));
    }

    if matches!(cli.command, Some(Commands::Update)) {
        return Ok(CliAction::Update);
    }

    if matches!(cli.command, Some(Commands::Check)) {
        return Ok(CliAction::Check);
    }

    if let Some(mml) = cli.mml {
        return Ok(CliAction::CliMml(mml));
    }

    Ok(CliAction::Tui)
}

fn print_help(help: &str) {
    print!("{}", help);
    if !help.ends_with('\n') {
        println!();
    }
    println!();
    match config::config_file_path() {
        Some(p) => println!("設定ファイル: {}", p.display()),
        None => println!("設定ファイル: (システムの設定ディレクトリが見つかりません)"),
    }
}

fn main() -> Result<()> {
    let action = parse_cli_from(std::env::args_os())?;

    if let CliAction::Help(help) = &action {
        print_help(help);
        return Ok(());
    }

    if let CliAction::Shutdown(port) = &action {
        server::shutdown_server(*port)?;
        println!(
            "サーバー（port {}）にシャットダウン要求を送りました。",
            port
        );
        return Ok(());
    }

    if matches!(&action, CliAction::Update) {
        if let Err(e) = server::shutdown_server(server::DEFAULT_PORT) {
            eprintln!(
                "サーバー停止要求の送信に失敗しました（port {}）: {}",
                server::DEFAULT_PORT,
                e
            );
        }
        return updater::run_foreground_update();
    }

    if matches!(&action, CliAction::Check) {
        return updater::run_check();
    }

    let cfg = config::load()?;

    // plugin_path が未設定の場合は設定ファイルを編集するよう案内する
    if cfg.plugin_path.is_empty() {
        let path_hint = match config::config_file_path() {
            Some(p) => p.display().to_string(),
            None => "(不明)".to_string(),
        };
        anyhow::bail!(
            "plugin_path が設定されていません。設定ファイルを編集して CLAP プラグインのパスを指定してください。\n設定ファイル: {}",
            path_hint
        );
    }

    let needs_plugin_entry = match action {
        CliAction::Server(_) | CliAction::CliMml(_) => true,
        CliAction::Tui => cfg.offline_render_backend == config::OfflineRenderBackend::InProcess,
        CliAction::Help(_) | CliAction::Shutdown(_) | CliAction::Update | CliAction::Check => {
            unreachable!()
        }
    };
    let entry = if needs_plugin_entry {
        Some(load_entry(&cfg.plugin_path)?)
    } else {
        None
    };

    match action {
        CliAction::Server(port) => {
            return server::run_server(
                &cfg,
                entry
                    .as_ref()
                    .expect("server mode must load a CLAP PluginEntry"),
                port,
            );
        }
        CliAction::CliMml(mml) => {
            println!("CLI モード: MML = {}", mml);
            let core_cfg = config::core_config_from_config(&cfg);
            let patch = mml_to_play(
                &mml,
                &core_cfg,
                entry
                    .as_ref()
                    .expect("CLI mode must load a CLAP PluginEntry"),
            )?;
            println!("patch: {}", patch);
            return Ok(());
        }
        CliAction::Tui => {}
        CliAction::Help(_) | CliAction::Shutdown(_) | CliAction::Update | CliAction::Check => {
            unreachable!()
        }
    }

    // TUI モード
    let mut app = tui::TuiApp::new(&cfg, entry.as_ref());

    match app.run()? {
        tui::TuiExitReason::Quit => Ok(()),
        tui::TuiExitReason::RestartApp => restart_current_process(),
    }
}

fn restart_current_process() -> Result<()> {
    let exe = std::env::current_exe().context("現在の実行ファイルパスを取得できませんでした")?;
    let args = std::env::args_os().skip(1).collect::<Vec<_>>();
    let status = std::process::Command::new(&exe)
        .args(args)
        .status()
        .with_context(|| format!("アプリの再起動に失敗しました: {}", exe.display()))?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!(
            "再起動したアプリが終了コード {} で終了しました",
            status
                .code()
                .map(|code| code.to_string())
                .unwrap_or_else(|| "不明".to_string())
        );
    }
}

#[cfg(test)]
#[path = "tests/main.rs"]
mod tests;
