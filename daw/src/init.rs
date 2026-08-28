use super::render_queue;
use super::save::load_saved_grid_size;
use super::*;
use cmrt_core::NativeRenderProbeContext;
use std::collections::HashMap;

mod cache_worker;
mod grid;

use cache_worker::{mark_cache_job_error, reserve_cache_job_for_render, store_cache_job_samples};
use grid::{build_grid_buffers_or_default, DawGridBuffers};

fn offline_render_startup_log_line(cfg: &cmrt_runtime::Config, render_workers: usize) -> String {
    format!(
        "offline render: backend={} workers={}",
        cfg.offline_render_backend.as_str(),
        render_workers
    )
}

fn realtime_audio_startup_log_line(cfg: &cmrt_runtime::Config) -> String {
    format!(
        "realtime audio: backend={}",
        cfg.realtime_audio_backend.as_str()
    )
}

pub(super) fn new(
    cfg: Arc<Config>,
    plugin_entries: cmrt_offline_render::PluginEntries,
    patch_load: Arc<Mutex<cmrt_tui_core::patch_load::PatchLoadState>>,
    realtime_play_supervisor: Option<Arc<cmrt_realtime_play::RealtimePlayServerSupervisor>>,
    workspace_kind: WorkspaceKind,
) -> DawApp {
    new_with_entry_context(
        cfg,
        plugin_entries,
        patch_load,
        realtime_play_supervisor,
        workspace_kind,
        cmrt_runtime::config_app_dir(),
        cmrt_tui_core::sound_check_guide::local_date_string(),
    )
}

fn new_with_entry_context(
    cfg: Arc<Config>,
    plugin_entries: cmrt_offline_render::PluginEntries,
    patch_load: Arc<Mutex<cmrt_tui_core::patch_load::PatchLoadState>>,
    realtime_play_supervisor: Option<Arc<cmrt_realtime_play::RealtimePlayServerSupervisor>>,
    workspace_kind: WorkspaceKind,
    config_app_dir: Option<std::path::PathBuf>,
    current_date: String,
) -> DawApp {
    super::http_server::set_active_http_state_context(Arc::clone(&cfg), Arc::clone(&patch_load));
    let DawGridBuffers {
        tracks,
        measures,
        data,
        cache,
        track_rerender_batches,
        play_measure_mmls,
        play_measure_track_mmls,
        play_track_gains,
        solo_tracks,
        track_volumes_db,
    } = build_grid_buffers_or_default(match workspace_kind {
        WorkspaceKind::Persistent => load_saved_grid_size(),
        WorkspaceKind::Daily => None,
    });

    let cache = Arc::new(Mutex::new(cache));

    let cache_render_workers = cfg.effective_offline_render_workers();
    let render_queue = RenderQueue::new(
        Arc::clone(&cfg),
        plugin_entries.clone(),
        cache_render_workers,
    );
    cmrt_tui_core::logging::install_native_probe_logger();

    // CacheJob は共通 RenderQueue に入り、MML -> SMF 前処理を 1 MML ずつ行う。
    // 準備済みジョブだけを render worker pool に流し、cache / preview / playback で
    // 同じ scheduler と render 並列度を共有する。
    let (cache_tx, cache_rx) = std::sync::mpsc::channel::<CacheJob>();
    let (cache_result_tx, cache_result_rx) =
        std::sync::mpsc::channel::<render_queue::RenderResult>();
    let pending_cache_jobs = Arc::new(Mutex::new(HashMap::<u64, CacheJob>::new()));
    let log_lines = Arc::new(Mutex::new(cmrt_tui_core::logging::load_log_lines()));
    let track_rerender_batches = Arc::new(Mutex::new(track_rerender_batches));
    let play_position = Arc::new(Mutex::new(None));
    let ab_repeat = Arc::new(Mutex::new(AbRepeatState::Off));
    let play_measure_mmls = Arc::new(Mutex::new(play_measure_mmls));
    let play_measure_track_mmls = Arc::new(Mutex::new(play_measure_track_mmls));
    let play_track_gains = Arc::new(Mutex::new(play_track_gains));
    // MML オーバーレイの発音は app から注入された supervisor を使う。DAW 自身の演奏用
    // supervisor（下の `realtime_play_server`）は backend が in_process だと `None` に
    // なるので、そちらを使うと実機ではオーバーレイが無音になる。
    let mml_overlay_sender = realtime_play_supervisor
        .map(|supervisor| cmrt_mml_overlay::MmlOverlaySender::new(supervisor, cfg.sample_rate));
    let realtime_play_server =
        if cfg.realtime_audio_backend == cmrt_runtime::RealtimeAudioBackend::PlayServer {
            Some(Arc::new(
                cmrt_realtime_play::RealtimePlayServerSupervisor::new(cfg.as_ref()),
            ))
        } else {
            None
        };

    {
        let cache_dispatch = Arc::clone(&cache);
        let render_queue = render_queue.clone();
        let cache_result_tx = cache_result_tx.clone();
        let pending_cache_jobs = Arc::clone(&pending_cache_jobs);
        let log_lines_dispatch = Arc::clone(&log_lines);
        let track_rerender_batches_dispatch = Arc::clone(&track_rerender_batches);
        let play_position_dispatch = Arc::clone(&play_position);
        let ab_repeat_dispatch = Arc::clone(&ab_repeat);
        let play_measure_mmls_dispatch = Arc::clone(&play_measure_mmls);
        let cache_tx_dispatch = cache_tx.clone();
        std::thread::spawn(move || {
            let rerender_completion_ctx = TrackRerenderBatchCompletionContext {
                batches: Arc::clone(&track_rerender_batches_dispatch),
                log_lines: Arc::clone(&log_lines_dispatch),
                cache: Arc::clone(&cache_dispatch),
                play_position: Arc::clone(&play_position_dispatch),
                ab_repeat: Arc::clone(&ab_repeat_dispatch),
                play_measure_mmls: Arc::clone(&play_measure_mmls_dispatch),
                cache_tx: cache_tx_dispatch.clone(),
                cache_render_workers,
            };

            while let Ok(job) = cache_rx.recv() {
                if !reserve_cache_job_for_render(&cache_dispatch, &job) {
                    DawApp::complete_track_rerender_batch_measure(
                        &rerender_completion_ctx,
                        job.track,
                        job.measure,
                    );
                    continue;
                }

                let request_id = render_queue.reserve_request_id();
                pending_cache_jobs
                    .lock()
                    .unwrap()
                    .insert(request_id, job.clone());
                let probe_context = NativeRenderProbeContext::cache_worker(
                    job.track,
                    job.measure,
                    job.generation,
                    job.rendered_mml_hash,
                    cache_render_workers,
                );
                if render_queue
                    .submit_with_id(
                        request_id,
                        render_queue::RenderPriority::Normal,
                        job.mml.clone(),
                        probe_context,
                        cache_result_tx.clone(),
                    )
                    .is_err()
                {
                    pending_cache_jobs.lock().unwrap().remove(&request_id);
                    mark_cache_job_error(&cache_dispatch, &job);
                    DawApp::complete_track_rerender_batch_measure(
                        &rerender_completion_ctx,
                        job.track,
                        job.measure,
                    );
                }
            }
        });
    }

    {
        let cache_result = Arc::clone(&cache);
        let cfg_result = Arc::clone(&cfg);
        let log_lines_result = Arc::clone(&log_lines);
        let track_rerender_batches_result = Arc::clone(&track_rerender_batches);
        let play_position_result = Arc::clone(&play_position);
        let ab_repeat_result = Arc::clone(&ab_repeat);
        let play_measure_mmls_result = Arc::clone(&play_measure_mmls);
        let cache_tx_result = cache_tx.clone();
        let pending_cache_jobs = Arc::clone(&pending_cache_jobs);
        std::thread::spawn(move || {
            let daw_cfg = (*cfg_result).clone();
            let rerender_completion_ctx = TrackRerenderBatchCompletionContext {
                batches: Arc::clone(&track_rerender_batches_result),
                log_lines: Arc::clone(&log_lines_result),
                cache: Arc::clone(&cache_result),
                play_position: Arc::clone(&play_position_result),
                ab_repeat: Arc::clone(&ab_repeat_result),
                play_measure_mmls: Arc::clone(&play_measure_mmls_result),
                cache_tx: cache_tx_result.clone(),
                cache_render_workers,
            };

            while let Ok(rendered) = cache_result_rx.recv() {
                let Some(job) = pending_cache_jobs
                    .lock()
                    .unwrap()
                    .remove(&rendered.request_id)
                else {
                    continue;
                };
                let track = job.track;
                let measure = job.measure;
                match rendered.result {
                    Ok(samples) => {
                        let _stored = store_cache_job_samples(
                            &cache_result,
                            &job,
                            &daw_cfg,
                            workspace_kind,
                            samples,
                        );
                        DawApp::complete_track_rerender_batch_measure(
                            &rerender_completion_ctx,
                            track,
                            measure,
                        );
                    }
                    Err(_) => {
                        mark_cache_job_error(&cache_result, &job);
                        DawApp::complete_track_rerender_batch_measure(
                            &rerender_completion_ctx,
                            track,
                            measure,
                        );
                    }
                }
            }
        });
    }

    let mut app = DawApp {
        workspace_kind,
        daily_page_date: None,
        config_app_dir,
        editor: super::DawEditorState::new(data, 0, 0, tracks, measures),
        mode: DawMode::Normal,
        help_origin: DawMode::Normal,
        sound_check_guide: cmrt_tui_core::sound_check_guide::SoundCheckGuide::new(None),
        textarea: cmrt_tui_core::text_input::new_single_line_textarea(""),
        cfg,
        plugin_entries,
        cache,
        cache_tx,
        cache_render_workers,
        render_queue,
        playback: super::DawPlaybackRuntime::new(
            realtime_play_server,
            play_position,
            ab_repeat,
            play_measure_mmls,
            play_measure_track_mmls,
            play_track_gains,
        ),
        log_lines,
        track_rerender_batches,
        solo_tracks,
        track_volumes_db,
        overlays: DawOverlays::new(FIRST_PLAYABLE_TRACK.min(tracks - 1)),
        patch_phrase_store: cmrt_history::load_patch_phrase_store(),
        patch_phrase_store_dirty: false,
        random_patch_decks: cmrt_tui_core::random::RandomIndexDecks::default(),
        chord_progression_source: None,
        patch_load,
        mml_overlay: cmrt_mml_overlay::MmlOverlay::default(),
        mml_overlay_sender,
    };

    app.load(&current_date);
    app.sync_http_grid_snapshot();
    app.sync_http_status_snapshot();
    app.append_log_line(offline_render_startup_log_line(
        &app.cfg,
        app.cache_render_workers,
    ));
    app.append_log_line(realtime_audio_startup_log_line(&app.cfg));
    app.append_log_line("=== DAW mode ready ===");
    app
}

#[cfg(test)]
mod tests;
