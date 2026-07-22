use anyhow::{Context, Result};
use chord2mml_core::convert as chord_to_mml;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use clap_mml_render_tui::{
    config, loop_browser::library as loop_library, server, tui, updater, voicing_cache_builder,
};
use cmrt_core::{load_entry, mml_to_play};

mod scan_progress_log;

#[derive(Debug, PartialEq, Eq)]
enum CliAction {
    Help(String),
    Version(String),
    Tui,
    CliMml(String),
    Server(u16),
    Shutdown(u16),
    Update,
    Check,
    ScanLoops,
    BuildVoicingCache { force: bool },
}

#[derive(Debug, PartialEq, Eq)]
enum CliPlaybackMml {
    Chord { chord: String, mml: String },
    Mml(String),
}

impl CliPlaybackMml {
    fn mml(&self) -> &str {
        match self {
            Self::Chord { mml, .. } | Self::Mml(mml) => mml,
        }
    }
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
    /// 全 patch の mono/poly を判定してキャッシュを作成する
    BuildVoicingCache {
        /// 判定済みの patch も含めて全件を再判定する
        #[arg(long)]
        force: bool,
    },
    /// loop_dirs を走査して WAV ループキャッシュを再構築する
    ScanLoops,
}

fn cli_command() -> clap::Command {
    Cli::command()
        .version(version_text())
        .after_help(format!(
            "サーバーモードでは HTTP POST でMMLを受け取りWAVデータを返します。\n  例: curl -X POST http://127.0.0.1:{}/ --data 'cde'",
            server::DEFAULT_PORT
        ))
}

fn version_text() -> String {
    format!(
        "{} (git {}, Rubber Band C API {} @ {})",
        env!("CARGO_PKG_VERSION"),
        env!("GIT_COMMIT_HASH"),
        rubberband_ffi::C_API_MAJOR_VERSION,
        rubberband_ffi::GIT_REVISION
    )
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
        Err(err) if err.kind() == clap::error::ErrorKind::DisplayVersion => {
            return Ok(CliAction::Version(err.to_string()));
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

    if matches!(cli.command, Some(Commands::ScanLoops)) {
        return Ok(CliAction::ScanLoops);
    }

    if let Some(Commands::BuildVoicingCache { force }) = cli.command {
        return Ok(CliAction::BuildVoicingCache { force });
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

fn cli_playback_mml(input: &str) -> CliPlaybackMml {
    match chord_to_mml(input) {
        Ok(mml) => CliPlaybackMml::Chord {
            chord: input.to_string(),
            mml,
        },
        Err(_) => CliPlaybackMml::Mml(input.to_string()),
    }
}

fn write_scan_progress(
    event: &loop_library::LoopScanProgress,
    stdout: &mut dyn std::io::Write,
    stderr: &mut dyn std::io::Write,
) -> std::io::Result<()> {
    match event {
        loop_library::LoopScanProgress::Started { roots } => {
            writeln!(stdout, "WAVループ走査を開始します: {roots} roots")?;
            stdout.flush()
        }
        loop_library::LoopScanProgress::Analyzing {
            current,
            total,
            path,
        } => {
            writeln!(stdout, "[{current}/{total}] WAVを解析: {}", path.display())?;
            stdout.flush()
        }
        loop_library::LoopScanProgress::Visualizing { .. } => Ok(()),
        loop_library::LoopScanProgress::Skipped { path, error } => {
            writeln!(
                stderr,
                "警告: WAVをスキップしました: {}\n  {error}",
                path.display()
            )?;
            stderr.flush()
        }
    }
}

fn write_scan_summary(
    summary: loop_library::LoopScanSummary,
    stdout: &mut dyn std::io::Write,
) -> std::io::Result<()> {
    writeln!(
        stdout,
        "ループキャッシュを更新しました: {} roots / {} indexed WAV / {} skipped WAV",
        summary.roots, summary.wav_files, summary.skipped_wav_files
    )?;
    stdout.flush()
}

fn run_scan_loops(cfg: &config::Config) -> Result<()> {
    let stdout = std::io::stdout();
    let stderr = std::io::stderr();
    let mut stdout = stdout.lock();
    let mut stderr = stderr.lock();
    let mut output_error = None;
    let log_path = config::scan_loops_log_file_path()
        .ok_or_else(|| anyhow::anyhow!("scan-loops logの保存先を取得できません"))?;
    let mut progress_log =
        scan_progress_log::ScanProgressLog::start(&log_path, std::time::Duration::from_secs(1))
            .with_context(|| format!("scan-loops logを開始できません: {}", log_path.display()))?;
    let scan_result = loop_library::scan_and_save_with_progress(cfg, |event| {
        progress_log.observe(&event);
        if output_error.is_none() {
            output_error = write_scan_progress(&event, &mut stdout, &mut stderr).err();
        }
    });
    let summary = match scan_result {
        Ok(summary) => summary,
        Err(error) => {
            let _ = progress_log.fail(&error);
            return Err(error);
        }
    };
    if let Some(error) = output_error {
        let _ = progress_log.fail(&error);
        return Err(error).context("scan-loopsの進捗を出力できません");
    }
    if let Err(error) = write_scan_summary(summary, &mut stdout) {
        let _ = progress_log.fail(&error);
        return Err(error).context("scan-loopsの完了結果を出力できません");
    }
    progress_log
        .finish(summary)
        .context("scan-loops logを完了できません")?;
    Ok(())
}

fn main() -> Result<()> {
    let action = parse_cli_from(std::env::args_os())?;

    if let CliAction::Help(help) = &action {
        print_help(help);
        return Ok(());
    }

    if let CliAction::Version(version) = &action {
        print!("{version}");
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

    if matches!(&action, CliAction::ScanLoops) {
        return run_scan_loops(&cfg);
    }

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
        // 判定は play-server 側プロセスがプラグインをロードして行う。
        CliAction::BuildVoicingCache { .. } => false,
        CliAction::Tui => cfg.offline_render_backend == config::OfflineRenderBackend::InProcess,
        CliAction::Help(_)
        | CliAction::Version(_)
        | CliAction::Shutdown(_)
        | CliAction::Update
        | CliAction::Check
        | CliAction::ScanLoops => {
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
            let playback_mml = cli_playback_mml(&mml);
            match &playback_mml {
                CliPlaybackMml::Chord { chord, mml } => {
                    println!("CLI モード: chord = {chord} / MML = {mml}");
                }
                CliPlaybackMml::Mml(mml) => {
                    println!("CLI モード: MML = {mml}");
                }
            }
            let core_cfg = config::core_config_from_config(&cfg);
            let patch = mml_to_play(
                playback_mml.mml(),
                &core_cfg,
                entry
                    .as_ref()
                    .expect("CLI mode must load a CLAP PluginEntry"),
            )?;
            println!("patch: {}", patch);
            return Ok(());
        }
        CliAction::BuildVoicingCache { force } => {
            return voicing_cache_builder::run_build_voicing_cache(&cfg, force);
        }
        CliAction::Tui => {}
        CliAction::Help(_)
        | CliAction::Version(_)
        | CliAction::Shutdown(_)
        | CliAction::Update
        | CliAction::Check
        | CliAction::ScanLoops => {
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
mod tests;
