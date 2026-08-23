use anyhow::{Context, Result};
use chord2mml_core::convert as chord_to_mml;
use clack_host::prelude::PluginEntry;
use clap::{CommandFactory, FromArgMatches, Parser, Subcommand};
use clap_mml_render_tui::{
    config, config_editor, render_mml, render_mml::RenderMmlRequest, server, tui, updater,
    voicing_cache_builder,
};
use cmrt_core::{load_entry, mml_to_play};
use std::path::PathBuf;

mod scan_loops;
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
    PatchRoles { config: Option<PathBuf> },
    RenderMml(RenderMmlRequest),
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
    /// MML 1 本をオフラインでレンダリングして出音を数字で出す（画面を起動しない動作確認）
    RenderMml {
        /// 既定の置き場ではなく、この config.toml を読む
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
        /// 鳴らす音色の display 文字列。複数指定すると 1 プロセスで順に鳴らして比べる
        #[arg(long = "patch", value_name = "DISPLAY")]
        patches: Vec<String>,
        /// 指定プラグインへ routing される共有カタログ上の全音色を鳴らす
        #[arg(long, value_name = "NAME", conflicts_with = "patches")]
        plugin: Option<String>,
        /// WAV の書き出し先（省略時は環境変数 CMRT_TEST_WAV_OUT_DIR。どちらも無ければ書かない）
        #[arg(long, value_name = "DIR")]
        out_dir: Option<PathBuf>,
        /// 和音が本当に和音で鳴るかを、単音のレンダリングと突き合わせて判定する
        #[arg(long)]
        poly_check: bool,
        /// 0件・誤 routing・load/render error・無音を失敗終了にする
        #[arg(long)]
        verify: bool,
        /// レンダリングする MML（省略時は 1 音だけ鳴らす既定 MML）
        #[arg(value_name = "MML")]
        mml: Option<String>,
    },
    /// grid sequencer の各行に patch の候補が出るかを調べる（画面を起動しない動作確認）
    PatchRoles {
        /// 既定の置き場ではなく、この config.toml を読む（実ユーザーの設定を書き換えずに
        /// `active_plugin` や `[plugins.*]` を試すため）
        #[arg(long, value_name = "PATH")]
        config: Option<PathBuf>,
    },
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

    if let Some(Commands::PatchRoles { config }) = cli.command {
        return Ok(CliAction::PatchRoles { config });
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
        return Ok(CliAction::RenderMml(RenderMmlRequest {
            config,
            patches,
            plugin,
            mml,
            out_dir,
            poly_check,
            verify,
        }));
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

fn main() -> Result<()> {
    // loop browser のデータ層（別 crate）に app ディレクトリ解決を注入する。
    clap_mml_render_tui::loop_browser::set_app_dir_resolver(config::config_app_dir);
    // loop browser 画面 crate にグローバルログ sink を注入する。
    cmrt_loop_browser::set_log_sinks(
        clap_mml_render_tui::logging::global_log_sink,
        clap_mml_render_tui::logging::nonblocking_log_sink,
    );
    // realtime play server crate にグローバルログ sink を注入する。
    cmrt_realtime_play::set_log_sink(clap_mml_render_tui::logging::global_log_sink);
    // offline render crate / notepad 画面 crate にグローバルログ sink を注入する。
    cmrt_offline_render::set_log_sink(clap_mml_render_tui::logging::global_log_sink);
    cmrt_notepad::set_log_sink(clap_mml_render_tui::logging::global_log_sink);
    // grid sequencer 画面 crate にグローバルログ sink を注入する。
    cmrt_grid_sequencer::set_log_sink(clap_mml_render_tui::logging::global_log_sink);
    // DAW 画面 crate / MML 入力オーバーレイ crate にグローバルログ sink を注入する。
    cmrt_daw::set_log_sink(clap_mml_render_tui::logging::global_log_sink);
    cmrt_mml_overlay::set_log_sink(clap_mml_render_tui::logging::global_log_sink);
    // DAW 画面 crate に config.toml 編集関数を注入する（terminal suspend は app ポリシー）。
    cmrt_daw::set_config_editor(config_editor::edit_config_toml);

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

    let cfg = match &action {
        // 診断コマンドだけは読む config を差し替えられる。既定の置き場を作りに行かないので、
        // 実ユーザーの config.toml には 1 バイトも触らない。
        CliAction::PatchRoles { config: Some(path) }
        | CliAction::RenderMml(RenderMmlRequest {
            config: Some(path), ..
        }) => cmrt_runtime::Config::load_from_path(path)?,
        _ => config::load()?,
    };

    // レンダリング結果キャッシュの置き場を、使用中プラグインごとに分ける。
    // キャッシュキーは MML 文字列の hash なので、音色を指定していない行は
    // プラグインを切り替えても同じキーになる。ここで名前空間を決めておく。
    cmrt_core::init_cache_plugin_namespace(&cfg.plugin_path);
    // 名前空間を切る前のバージョンが残したキャッシュは、誰も読まないのに
    // LRU の上限計算からも外れて消えなくなるので、ここで捨てる。
    cmrt_core::remove_legacy_unnamespaced_caches();

    if matches!(&action, CliAction::ScanLoops) {
        return scan_loops::run_scan_loops(&cfg);
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
        // config と patch 一覧だけを見る診断なので、プラグインはロードしない。
        CliAction::PatchRoles { .. } => false,
        // in-process バックエンドのときだけ、この プロセスが CLAP をホストする。
        CliAction::RenderMml(_) | CliAction::Tui => {
            cfg.offline_render_backend == config::OfflineRenderBackend::InProcess
        }
        CliAction::Help(_)
        | CliAction::Version(_)
        | CliAction::Shutdown(_)
        | CliAction::Update
        | CliAction::Check
        | CliAction::ScanLoops => {
            unreachable!()
        }
    };
    // カタログに音色を載せるプラグインぶんの entry をロードする。並びは
    // `catalog_plugins` と同じで、先頭が既定プラグイン。オフラインレンダリングは
    // MML が指す音色でこの中から引き分ける（`docs/adr/0009-offline-entry-map.md`）。
    // server / CLI 経路が使うのは先頭の 1 本だけ。
    let entries: Vec<PluginEntry> = if needs_plugin_entry {
        config::catalog_plugins(&cfg)
            .iter()
            .map(|plugin| load_entry(&plugin.plugin_path))
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    let plugin_entries = cmrt_offline_render::PluginEntries::from_loaded(&entries);
    // MML 1 本ごとに「その音色のプラグイン」を引く表。server / CLI もこれを通す。
    let in_process_plugins = cmrt_offline_render::InProcessPlugins::new(&cfg, &plugin_entries);

    match action {
        CliAction::Server(port) => {
            return server::run_server(&cfg, &in_process_plugins, port);
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
            let (entry, core_cfg) = in_process_plugins.for_mml(playback_mml.mml())?;
            let patch = mml_to_play(playback_mml.mml(), core_cfg, entry)?;
            println!("patch: {}", patch);
            return Ok(());
        }
        CliAction::BuildVoicingCache { force } => {
            return voicing_cache_builder::run_build_voicing_cache(&cfg, force);
        }
        CliAction::RenderMml(request) => {
            return render_mml::run(&cfg, &plugin_entries, &request);
        }
        CliAction::PatchRoles { .. } => {
            return tui::patch_role_report::run_patch_role_report(&cfg);
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
    let mut app = tui::TuiApp::new(&cfg, plugin_entries);

    let exit_reason = app.run()?;
    drop(app);
    match exit_reason {
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
