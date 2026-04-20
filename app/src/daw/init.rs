use super::render_queue;
use super::save::load_saved_grid_size;
use super::*;
use cmrt_core::NativeRenderProbeContext;
use std::collections::HashMap;

struct DawGridBuffers {
    tracks: usize,
    measures: usize,
    data: Vec<Vec<String>>,
    cache: Vec<Vec<CellCache>>,
    track_rerender_batches: Vec<Option<TrackRerenderBatch>>,
    play_measure_mmls: Vec<String>,
    play_measure_track_mmls: Vec<Vec<String>>,
    play_track_gains: Vec<f32>,
    solo_tracks: Vec<bool>,
    track_volumes_db: Vec<i32>,
}

fn try_build_string_row(len: usize) -> Option<Vec<String>> {
    let mut row = Vec::new();
    row.try_reserve_exact(len).ok()?;
    row.resize_with(len, String::new);
    Some(row)
}

fn try_build_cache_row(len: usize) -> Option<Vec<CellCache>> {
    let mut row = Vec::new();
    row.try_reserve_exact(len).ok()?;
    row.resize_with(len, CellCache::empty);
    Some(row)
}

fn try_build_string_grid(rows: usize, cols: usize) -> Option<Vec<Vec<String>>> {
    let mut grid = Vec::new();
    grid.try_reserve_exact(rows).ok()?;
    for _ in 0..rows {
        grid.push(try_build_string_row(cols)?);
    }
    Some(grid)
}

fn try_build_cache_grid(rows: usize, cols: usize) -> Option<Vec<Vec<CellCache>>> {
    let mut grid = Vec::new();
    grid.try_reserve_exact(rows).ok()?;
    for _ in 0..rows {
        grid.push(try_build_cache_row(cols)?);
    }
    Some(grid)
}

fn try_build_none_vec<T>(len: usize) -> Option<Vec<Option<T>>> {
    let mut values = Vec::new();
    values.try_reserve_exact(len).ok()?;
    values.resize_with(len, || None);
    Some(values)
}

fn try_build_default_vec<T: Clone>(len: usize, value: T) -> Option<Vec<T>> {
    let mut values = Vec::new();
    values.try_reserve_exact(len).ok()?;
    values.resize(len, value);
    Some(values)
}

fn try_build_grid_buffers(tracks: usize, measures: usize) -> Option<DawGridBuffers> {
    let columns = measures.checked_add(1)?;
    let _data_cells = tracks.checked_mul(columns)?;
    let _play_measure_cells = measures.checked_mul(tracks)?;

    let mut data = try_build_string_grid(tracks, columns)?;
    data[0][0] = DEFAULT_TRACK0_MML.to_string();

    Some(DawGridBuffers {
        tracks,
        measures,
        data,
        cache: try_build_cache_grid(tracks, columns)?,
        track_rerender_batches: try_build_none_vec(tracks)?,
        play_measure_mmls: try_build_string_row(measures)?,
        play_measure_track_mmls: try_build_string_grid(measures, tracks)?,
        play_track_gains: try_build_default_vec(tracks, 0.0)?,
        solo_tracks: try_build_default_vec(tracks, false)?,
        track_volumes_db: try_build_default_vec(tracks, 0)?,
    })
}

fn build_grid_buffers_or_default(saved_grid_dimensions: Option<(usize, usize)>) -> DawGridBuffers {
    let (requested_tracks, requested_measures) = saved_grid_dimensions
        .map(|(tracks, measures)| (TRACKS.max(tracks), MEASURES.max(measures)))
        .unwrap_or((TRACKS, MEASURES));

    if let Some(buffers) = try_build_grid_buffers(requested_tracks, requested_measures) {
        return buffers;
    }

    // DAW アプリ本体はまだ未構築なので append_log_line() は使えず、ここでは stderr にフォールバックする。
    eprintln!(
        "DAW セッションのサイズが大きすぎるか破損しているため、デフォルトサイズ {}x{} にフォールバックします。",
        TRACKS, MEASURES
    );
    try_build_grid_buffers(TRACKS, MEASURES)
        .expect("default DAW grid should be allocatable in supported environments")
}

fn reserve_cache_job_for_render(cache: &Arc<Mutex<Vec<Vec<CellCache>>>>, job: &CacheJob) -> bool {
    let mut cache = cache.lock().unwrap();
    let cell = &mut cache[job.track][job.measure];
    if cell.state == CacheState::Empty || cell.generation != job.generation {
        return false;
    }
    cell.state = CacheState::Rendering;
    cell.rendered_mml_hash = None;
    true
}

fn mark_cache_job_error(cache: &Arc<Mutex<Vec<Vec<CellCache>>>>, job: &CacheJob) {
    let mut cache = cache.lock().unwrap();
    if cache[job.track][job.measure].generation != job.generation {
        return;
    }
    cache[job.track][job.measure].state = CacheState::Error;
    cache[job.track][job.measure].samples = None;
    cache[job.track][job.measure].rendered_measure_samples = None;
    cache[job.track][job.measure].rendered_mml_hash = None;
}

fn store_cache_job_samples(
    cache: &Arc<Mutex<Vec<Vec<CellCache>>>>,
    job: &CacheJob,
    daw_cfg: &crate::config::Config,
    samples: Vec<f32>,
) -> bool {
    let mut cache = cache.lock().unwrap();
    if cache[job.track][job.measure].generation != job.generation {
        return false;
    }

    // 開発用: track/measure ごとに WAV ファイルを出力する。
    // measure 0 は音色/ヘッダセルであり演奏内容ではないためスキップ。
    let wav_ok = if job.measure > 0 {
        if let Ok(daw_dir) = ensure_daw_dir() {
            let wav_path = daw_dir.join(format!("track{}_meas{}.wav", job.track, job.measure));
            write_wav(&samples, daw_cfg.sample_rate as u32, &wav_path).is_ok()
        } else {
            false
        }
    } else {
        true
    };

    let cell = &mut cache[job.track][job.measure];
    cell.state = if wav_ok {
        CacheState::Ready
    } else {
        CacheState::Error
    };
    cell.rendered_mml_hash = if wav_ok {
        Some(job.rendered_mml_hash)
    } else {
        None
    };
    // Ready かつサイズ上限以内のときのみサンプルをメモリに保持する。
    // 上限超過（低 BPM 等）や WAV 失敗時はサンプルを保持しない。
    if wav_ok && samples.len() <= MAX_CACHED_SAMPLES {
        cell.samples = Some(Arc::new(samples));
        cell.rendered_measure_samples = Some(job.measure_samples);
    } else {
        cell.samples = None;
        cell.rendered_measure_samples = None;
    }
    true
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

    let cache_render_workers = cfg.offline_render_workers;
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
        patch_favorites_cursor: 0,
        patch_select_focus: DawPatchSelectPane::Patches,
        patch_select_filter_active: false,
        random_patch_decks: crate::random::RandomIndexDecks::default(),
    };

    app.load();
    app.sync_http_grid_snapshot();
    app.sync_http_status_snapshot();
    app.append_log_line("=== DAW mode ready ===");
    app
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn try_build_grid_buffers_rejects_measure_overflow() {
        assert!(try_build_grid_buffers(2, usize::MAX).is_none());
    }

    #[test]
    fn build_grid_buffers_or_default_falls_back_from_invalid_saved_size() {
        let buffers = build_grid_buffers_or_default(Some((usize::MAX, usize::MAX)));

        assert_eq!(buffers.tracks, TRACKS);
        assert_eq!(buffers.measures, MEASURES);
        assert_eq!(buffers.data.len(), TRACKS);
        assert_eq!(buffers.data[0].len(), MEASURES + 1);
    }
}
