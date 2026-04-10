use super::save::load_saved_grid_size;
use super::*;

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

pub(super) fn new(cfg: Arc<Config>, entry_ptr: usize) -> DawApp {
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

    // 固定数のキャッシュワーカースレッドを起動する。
    // MML -> SMF の前処理排他は core-lib 側で行い、ここでは render 本体の並列度だけを増やす。
    let (cache_tx, cache_rx) = std::sync::mpsc::channel::<CacheJob>();
    let cache_rx = Arc::new(Mutex::new(cache_rx));
    let log_lines = Arc::new(Mutex::new(crate::logging::load_log_lines()));
    let track_rerender_batches = Arc::new(Mutex::new(track_rerender_batches));
    let play_position = Arc::new(Mutex::new(None));
    let ab_repeat = Arc::new(Mutex::new(AbRepeatState::Off));
    let play_measure_mmls = Arc::new(Mutex::new(play_measure_mmls));
    let play_measure_track_mmls = Arc::new(Mutex::new(play_measure_track_mmls));
    let play_track_gains = Arc::new(Mutex::new(play_track_gains));

    for _ in 0..CACHE_RENDER_WORKERS {
        let cache_worker = Arc::clone(&cache);
        let cache_rx_worker = Arc::clone(&cache_rx);
        let cfg_worker = Arc::clone(&cfg);
        let log_lines_worker = Arc::clone(&log_lines);
        let track_rerender_batches_worker = Arc::clone(&track_rerender_batches);
        let play_position_worker = Arc::clone(&play_position);
        let ab_repeat_worker = Arc::clone(&ab_repeat);
        let play_measure_mmls_worker = Arc::clone(&play_measure_mmls);
        let cache_tx_worker = cache_tx.clone();
        std::thread::spawn(move || {
            // SAFETY: entry は main() のスタックに生存している
            let entry_ref: &PluginEntry = unsafe { &*(entry_ptr as *const PluginEntry) };
            let daw_cfg = (*cfg_worker).clone();
            let rerender_completion_ctx = TrackRerenderBatchCompletionContext {
                batches: Arc::clone(&track_rerender_batches_worker),
                log_lines: Arc::clone(&log_lines_worker),
                cache: Arc::clone(&cache_worker),
                play_position: Arc::clone(&play_position_worker),
                ab_repeat: Arc::clone(&ab_repeat_worker),
                play_measure_mmls: Arc::clone(&play_measure_mmls_worker),
                cache_tx: cache_tx_worker.clone(),
            };

            loop {
                let job = {
                    let rx = cache_rx_worker.lock().unwrap();
                    match rx.recv() {
                        Ok(job) => job,
                        Err(_) => break,
                    }
                };
                let track = job.track;
                let measure = job.measure;
                let mut skipped_stale_job = false;
                {
                    let mut cache = cache_worker.lock().unwrap();
                    let cell = &mut cache[track][measure];
                    if cell.state == CacheState::Empty || cell.generation != job.generation {
                        skipped_stale_job = true;
                    } else {
                        cell.state = CacheState::Rendering;
                        cell.rendered_mml_hash = None;
                    }
                }
                if skipped_stale_job {
                    DawApp::complete_track_rerender_batch_measure(
                        &rerender_completion_ctx,
                        track,
                        measure,
                    );
                    continue;
                }
                let core_cfg = cmrt_core::CoreConfig::from(&daw_cfg);
                match mml_render_for_cache(&job.mml, &core_cfg, entry_ref) {
                    Ok(samples) => {
                        let mut should_complete_batch = false;
                        {
                            let mut cache = cache_worker.lock().unwrap();
                            if cache[track][measure].generation != job.generation {
                                skipped_stale_job = true;
                            } else {
                                // 開発用: track/measure ごとに WAV ファイルを出力する
                                // measure 0 は音色/ヘッダセルであり演奏内容ではないためスキップ
                                let wav_ok = if measure > 0 {
                                    if let Ok(daw_dir) = ensure_daw_dir() {
                                        let wav_path = daw_dir
                                            .join(format!("track{}_meas{}.wav", track, measure));
                                        write_wav(&samples, daw_cfg.sample_rate as u32, &wav_path)
                                            .is_ok()
                                    } else {
                                        false
                                    }
                                } else {
                                    true
                                };
                                // WAV 書き出し失敗はデバッグ出力の問題であり、レンダリング自体は成功している。
                                // そのため WAV 失敗時は Error としてユーザーに通知する。
                                cache[track][measure].state = if wav_ok {
                                    CacheState::Ready
                                } else {
                                    CacheState::Error
                                };
                                cache[track][measure].rendered_mml_hash = if wav_ok {
                                    Some(job.rendered_mml_hash)
                                } else {
                                    None
                                };
                                // Ready かつサイズ上限以内のときのみサンプルをメモリに保持する。
                                // 上限超過（低 BPM 等）や WAV 失敗時はサンプルを保持しない。
                                if wav_ok && samples.len() <= MAX_CACHED_SAMPLES {
                                    cache[track][measure].samples = Some(Arc::new(samples));
                                    cache[track][measure].rendered_measure_samples =
                                        Some(job.measure_samples);
                                } else {
                                    cache[track][measure].samples = None;
                                    cache[track][measure].rendered_measure_samples = None;
                                }
                                should_complete_batch = true;
                            }
                        }
                        if skipped_stale_job || should_complete_batch {
                            DawApp::complete_track_rerender_batch_measure(
                                &rerender_completion_ctx,
                                track,
                                measure,
                            );
                        }
                    }
                    Err(_) => {
                        let mut should_complete_batch = false;
                        {
                            let mut cache = cache_worker.lock().unwrap();
                            if cache[track][measure].generation != job.generation {
                                skipped_stale_job = true;
                            } else {
                                cache[track][measure].state = CacheState::Error;
                                // エラー時は古いサンプルを保持しない（ステールデータの排除）
                                cache[track][measure].samples = None;
                                cache[track][measure].rendered_measure_samples = None;
                                cache[track][measure].rendered_mml_hash = None;
                                should_complete_batch = true;
                            }
                        }
                        if skipped_stale_job || should_complete_batch {
                            DawApp::complete_track_rerender_batch_measure(
                                &rerender_completion_ctx,
                                track,
                                measure,
                            );
                        }
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
        textarea: TextArea::default(),
        cfg,
        entry_ptr,
        tracks,
        measures,
        cache,
        cache_tx,
        play_state: Arc::new(Mutex::new(DawPlayState::Idle)),
        play_transition_lock: Arc::new(Mutex::new(())),
        preview_session: Arc::new(AtomicU64::new(0)),
        preview_sink: Arc::new(Mutex::new(None)),
        play_position,
        ab_repeat,
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
    };

    app.load();
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
