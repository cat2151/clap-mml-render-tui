use super::render_queue;
use super::save::load_saved_grid_size;
use super::*;
use cmrt_core::NativeRenderProbeContext;
use std::collections::HashMap;

mod cache_worker;
mod grid;

use cache_worker::{mark_cache_job_error, reserve_cache_job_for_render, store_cache_job_samples};
use grid::{build_grid_buffers_or_default, DawGridBuffers};

fn offline_render_startup_log_line(cfg: &crate::config::Config, render_workers: usize) -> String {
    format!(
        "offline render: backend={} workers={}",
        cfg.offline_render_backend.as_str(),
        render_workers
    )
}

fn realtime_audio_startup_log_line(cfg: &crate::config::Config) -> String {
    format!(
        "realtime audio: backend={}",
        cfg.realtime_audio_backend.as_str()
    )
}

pub(super) fn new(cfg: Arc<Config>, entry_ptr: usize) -> DawApp {
    super::http_server::set_active_http_state_cfg(Arc::clone(&cfg));
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
    } = build_grid_buffers_or_default(load_saved_grid_size());

    let cache = Arc::new(Mutex::new(cache));

    let cache_render_workers = cfg.effective_offline_render_workers();
    let render_queue = RenderQueue::new(Arc::clone(&cfg), entry_ptr, cache_render_workers);
    crate::logging::install_native_probe_logger();

    // CacheJob は共通 RenderQueue に入り、MML -> SMF 前処理を 1 MML ずつ行う。
    // 準備済みジョブだけを render worker pool に流し、cache / preview / playback で
    // 同じ scheduler と render 並列度を共有する。
    let (cache_tx, cache_rx) = std::sync::mpsc::channel::<CacheJob>();
    let (cache_result_tx, cache_result_rx) =
        std::sync::mpsc::channel::<render_queue::RenderResult>();
    let pending_cache_jobs = Arc::new(Mutex::new(HashMap::<u64, CacheJob>::new()));
    let log_lines = Arc::new(Mutex::new(crate::logging::load_log_lines()));
    let track_rerender_batches = Arc::new(Mutex::new(track_rerender_batches));
    let play_position = Arc::new(Mutex::new(None));
    let ab_repeat = Arc::new(Mutex::new(AbRepeatState::Off));
    let play_measure_mmls = Arc::new(Mutex::new(play_measure_mmls));
    let play_measure_track_mmls = Arc::new(Mutex::new(play_measure_track_mmls));
    let play_track_gains = Arc::new(Mutex::new(play_track_gains));
    let realtime_play_server =
        if cfg.realtime_audio_backend == crate::config::RealtimeAudioBackend::PlayServer {
            Some(Arc::new(
                crate::realtime_play::RealtimePlayServerSupervisor::new(cfg.as_ref()),
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
                        let _stored =
                            store_cache_job_samples(&cache_result, &job, &daw_cfg, samples);
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
        data,
        cursor_track: 0,
        cursor_measure: 0,
        mode: DawMode::Normal,
        help_origin: DawMode::Normal,
        textarea: crate::text_input::new_single_line_textarea(""),
        cfg,
        entry_ptr,
        tracks,
        measures,
        cache,
        cache_tx,
        cache_render_workers,
        render_queue,
        play_state: Arc::new(Mutex::new(DawPlayState::Idle)),
        play_transition_lock: Arc::new(Mutex::new(())),
        preview_session: Arc::new(AtomicU64::new(0)),
        preview_sink: Arc::new(Mutex::new(None)),
        realtime_play_server,
        play_position,
        ab_repeat,
        overlay_preview_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
        play_measure_mmls,
        play_measure_track_mmls,
        play_measure_samples: Arc::new(Mutex::new(0)),
        log_lines,
        track_rerender_batches,
        solo_tracks,
        track_volumes_db,
        mixer_cursor_track: FIRST_PLAYABLE_TRACK.min(tracks - 1),
        play_track_gains,
        yank_buffer: None,
        normal_pending_delete: false,
        normal_paste_undo: None,
        patch_phrase_store: crate::history::load_patch_phrase_store(),
        patch_phrase_store_dirty: false,
        history_overlay_patch_name: None,
        history_overlay_query: String::new(),
        history_overlay_query_textarea: crate::text_input::new_single_line_textarea(""),
        history_overlay_history_cursor: 0,
        history_overlay_favorites_cursor: 0,
        history_overlay_focus: DawHistoryPane::History,
        history_overlay_filter_active: false,
        patch_all: Vec::new(),
        patch_query: String::new(),
        patch_query_textarea: crate::text_input::new_single_line_textarea(""),
        patch_query_before_input: String::new(),
        patch_filtered: Vec::new(),
        patch_cursor: 0,
        patch_favorite_items: Vec::new(),
        patch_favorites_query: String::new(),
        patch_favorites_query_textarea: crate::text_input::new_single_line_textarea(""),
        patch_favorites_query_before_input: String::new(),
        patch_favorites_cursor: 0,
        patch_select_focus: DawPatchSelectPane::Patches,
        patch_select_filter_active: false,
        random_patch_decks: crate::random::RandomIndexDecks::default(),
    };

    app.load();
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
#[path = "init/tests.rs"]
mod tests;
