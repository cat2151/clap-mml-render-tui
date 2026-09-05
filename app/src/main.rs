use anyhow::Result;
use clack_host::prelude::PluginEntry;
use clap_mml_render_tui::{
    config, config_editor, render_mml, render_mml::RenderMmlRequest, server, tui, updater,
    voicing_cache_builder,
};
use cmrt_core::{load_entry, mml_to_play};

mod cli;
mod cli_output;
mod cli_playback;
mod process_restart;
mod scan_loops;
mod scan_progress_log;

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

    let mut cfg = match &action {
        // 診断コマンドだけは読む config を差し替えられる。既定の置き場を作りに行かないので、
        // 実ユーザーの config.toml には 1 バイトも触らない。
        CliAction::PatchRoles { config: Some(path) }
        | CliAction::RenderMml(RenderMmlRequest {
            config: Some(path), ..
        }) => cmrt_runtime::Config::load_from_path(path)?,
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
             first_load_failures={} second_load_failures={} catalog_voicings={} catalog_unknown={} path={}",
            summary.patch_count,
            summary.plugin_names.join(","),
            summary.measured_load_count,
            summary.first_load_failure_count,
            summary.second_load_failure_count,
            summary.catalog_voicing_count,
            summary.catalog_unknown_count,
            summary.path.display()
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

    let needs_plugin_entry = match action {
        CliAction::Server(_) | CliAction::CliMml(_) => true,
        // 判定は play-server 側プロセスがプラグインをロードして行う。
        CliAction::BuildVoicingCache { .. } => false,
        CliAction::BuildPatchCatalogCache => false,
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
    let catalog = if needs_plugin_entry && !matches!(action, CliAction::Tui) {
        config::catalog_plugins(&cfg)
    } else {
        Vec::new()
    };
    let entries: Vec<PluginEntry> = if !catalog.is_empty() {
        catalog
            .iter()
            .map(|plugin| load_entry(&plugin.plugin_path))
            .collect::<Result<Vec<_>>>()?
    } else {
        Vec::new()
    };
    let plugin_entries = if matches!(action, CliAction::Tui)
        && cfg.offline_render_backend == config::OfflineRenderBackend::InProcess
    {
        cmrt_offline_render::PluginEntries::pending()
    } else if !catalog.is_empty() {
        cmrt_offline_render::PluginEntries::from_loaded(catalog, &entries)
    } else {
        cmrt_offline_render::PluginEntries::none()
    };
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
            let patch = mml_to_play(playback_mml.mml(), &core_cfg, &entry)?;
            println!("patch: {}", patch);
            return Ok(());
        }
        CliAction::BuildVoicingCache { force } => {
            return voicing_cache_builder::run_build_voicing_cache(&cfg, force);
        }
        CliAction::BuildPatchCatalogCache => unreachable!(),
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
        tui::TuiExitReason::RestartApp => process_restart::restart_current_process(),
    }
}
