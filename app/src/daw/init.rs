use super::*;

pub(super) fn new(cfg: Arc<Config>, entry_ptr: usize) -> DawApp {
    let tracks = cfg.daw_tracks.clamp(2, 64);
    let measures = cfg.daw_measures.clamp(1, 64);
    let mut data = vec![vec![String::new(); measures + 1]; tracks];
    // track 0 のデフォルトは拍子指定 JSON + テンポ設定
    data[0][0] = DEFAULT_TRACK0_MML.to_string();

    let cache = Arc::new(Mutex::new(vec![
        vec![CellCache::empty(); measures + 1];
        tracks
    ]));

    // シリアルなキャッシュワーカースレッドを起動する。
    // チャネルが送信側（cache_tx）を介してジョブを受け取り順次レンダリングすることで
    // ファイル書き込み（clap-mml-render-tui/pass1_tokens.json 等）の競合と過剰スレッド生成を防ぐ。
    let (cache_tx, cache_rx) = std::sync::mpsc::channel::<CacheJob>();

    // `mml_render_for_cache` はキャッシュワーカーと再生スレッドの両方から呼ばれるため、
    // `mml_str_to_smf_bytes` が書き出す共有デバッグファイル
    // （`pass1_tokens.json` など）への同時書き込みを防ぐ排他ロックを共有する。
    let render_lock: Arc<Mutex<()>> = Arc::new(Mutex::new(()));
    let log_lines = Arc::new(Mutex::new(crate::logging::load_log_lines()));
    let track_rerender_batches = Arc::new(Mutex::new(vec![None; tracks]));
    let play_position = Arc::new(Mutex::new(None));
    let ab_repeat = Arc::new(Mutex::new(AbRepeatState::Off));
    let play_measure_mmls = Arc::new(Mutex::new(vec![String::new(); measures]));
    let play_measure_track_mmls = Arc::new(Mutex::new(vec![vec![String::new(); tracks]; measures]));
    let play_track_gains = Arc::new(Mutex::new(vec![0.0; tracks]));

    {
        let cache_worker = Arc::clone(&cache);
        let cfg_worker = Arc::clone(&cfg);
        let render_lock_worker = Arc::clone(&render_lock);
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

            for job in cache_rx {
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
                let _guard = render_lock_worker.lock().unwrap();
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
        render_lock,
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
        solo_tracks: vec![false; tracks],
        track_volumes_db: vec![0; tracks],
        mixer_cursor_track: FIRST_PLAYABLE_TRACK.min(tracks - 1),
        play_track_gains,
        yank_buffer: None,
        normal_pending_delete: false,
        patch_phrase_store: crate::history::load_patch_phrase_store(),
        patch_phrase_store_dirty: false,
        history_overlay_patch_name: None,
        history_overlay_query: String::new(),
        history_overlay_history_cursor: 0,
        history_overlay_favorites_cursor: 0,
        history_overlay_focus: DawHistoryPane::History,
        history_overlay_filter_active: false,
        patch_all: Vec::new(),
        patch_query: String::new(),
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
