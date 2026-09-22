use anyhow::Result;
use clap_mml_render_tui::{
    bass_voicing_inspect, config, config_editor, live_chord_check,
    live_chord_check::LiveChordCheckRequest, render_mml, render_mml::RenderMmlRequest, server, tui,
    updater, voicing_cache_builder,
};
use cmrt_core::play_samples;

mod cli;
mod cli_output;
mod cli_playback;
mod process_restart;
mod scan_loops;
mod scan_progress_log;

use std::sync::Arc;

use cli::{parse_cli_invocation_from, play_server_launch, CliAction, CliInvocation};
use cli_playback::{cli_playback_mml, CliPlaybackMml};

fn main() -> Result<()> {
    clap_mml_render_tui::logging::install_panic_log_hook();
    let result = run();
    if let Err(error) = &result {
        clap_mml_render_tui::logging::global_log_sink(&format!(
            "app: event=fatal-error error={:?}",
            format!("{error:#}")
        ));
    }
    result
}

fn run() -> Result<()> {
    clap_mml_render_tui::logging::install_embedded_core_log_sink();
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
    cmrt_daw::set_performance_log_sink(clap_mml_render_tui::logging::nonblocking_log_sink);
    cmrt_mml_overlay::set_log_sink(clap_mml_render_tui::logging::global_log_sink);
    // DAW 画面 crate に config.toml 編集関数を注入する（terminal suspend は app ポリシー）。
    cmrt_daw::set_config_editor(config_editor::edit_config_toml);

    let CliInvocation {
        action,
        play_server,
    } = parse_cli_invocation_from(std::env::args_os())?;

    if let CliAction::Help(help) = &action {
        cli_output::print_help(help);
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

    // 音源・configを一切使わない純粋なvoicing診断。空のconfig環境でも実行できるよう、
    // 通常起動の初期化より前に返す。
    if let CliAction::InspectBassVoicing(request) = &action {
        print!("{}", bass_voicing_inspect::report(request)?);
        return Ok(());
    }

    let mut cfg = match &action {
        // 診断コマンドだけは読む config を差し替えられる。既定の置き場を作りに行かないので、
        // 実ユーザーの config.toml には 1 バイトも触らない。
        CliAction::PatchRoles { config: Some(path) }
        | CliAction::RenderMml(RenderMmlRequest {
            config: Some(path), ..
        })
        | CliAction::LiveChordCheck(LiveChordCheckRequest {
            config: Some(path), ..
        }) => {
            let mut cfg = cmrt_runtime::Config::load_from_path(path)?;
            cfg.source_path = Some(path.clone());
            cfg
        }
        _ => config::load()?,
    };
    // 明示指定は探索より強い。存在しなければここで止める（探索へ落とさない）。
    cfg.play_server_launch_override = play_server.map(play_server_launch).transpose()?;

    // レンダリング結果キャッシュの置き場を、使用中プラグインごとに分ける。
    // キャッシュキーは MML 文字列の hash なので、音色を指定していない行は
    // プラグインを切り替えても同じキーになる。ここで名前空間を決めておく。
    cmrt_core::init_cache_plugin_namespace(&cfg.plugin_path);
    // 旧配置のキャッシュを現在の配置へ移行し、再利用できないものは掃除する。
    cmrt_core::migrate_legacy_caches();

    if matches!(&action, CliAction::ScanLoops) {
        return scan_loops::run_scan_loops(&cfg);
    }

    if matches!(&action, CliAction::BuildPatchCatalogCache) {
        let summary = clap_mml_render_tui::patch_catalog_cache::build_and_save(&cfg)?;
        println!(
            "patch catalog cacheを構築しました: patches={} plugins={} measured_loads={} \
             first_load_failures={} second_load_failures={} catalog_voicings={} catalog_unknown={} path={} source_path={}",
            summary.patch_count,
            summary.plugin_names.join(","),
            summary.measured_load_count,
            summary.first_load_failure_count,
            summary.second_load_failure_count,
            summary.catalog_voicing_count,
            summary.catalog_unknown_count,
            summary.path.display(),
            summary.source_path.display()
        );
        return Ok(());
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

    // オフラインレンダリングはすべて render-server 子プロセスへ投げる。このプロセスは
    // CLAP をロードしない。
    let offline_renderer = || cmrt_offline_render::OfflineRenderer::new(Arc::new(cfg.clone()));

    match action {
        CliAction::Server(port) => {
            return server::run_server(&cfg, &offline_renderer(), port);
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
            let rendered = offline_renderer().render_phrase(playback_mml.mml())?;
            play_samples(rendered.samples, cfg.sample_rate as u32)?;
            println!("patch: {}", rendered.patch_name);
            return Ok(());
        }
        CliAction::BuildVoicingCache { force } => {
            return voicing_cache_builder::run_build_voicing_cache(&cfg, force);
        }
        CliAction::BuildPatchCatalogCache => unreachable!(),
        CliAction::RenderMml(request) => {
            return render_mml::run(&cfg, &request);
        }
        CliAction::LiveChordCheck(request) => {
            return live_chord_check::run(&cfg, &request);
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
        | CliAction::InspectBassVoicing(_)
        | CliAction::ScanLoops => {
            unreachable!()
        }
    }

    // TUI モード。EFFECT CHAIN overlay（`x`）が catalog を一覧できるよう effect を discover
    // する（preset ファイルの走査だけで、DLL はロードしない）。
    let mut app = tui::TuiApp::new(&cfg, cmrt_offline_render::EffectPlugins::discover());

    let exit_reason = app.run()?;
    drop(app);
    match exit_reason {
        tui::TuiExitReason::Quit => Ok(()),
        tui::TuiExitReason::RestartApp => process_restart::restart_current_process(),
    }
}
