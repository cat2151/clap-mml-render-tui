use super::*;

#[test]
fn complete_track_rerender_batch_uses_available_worker_slots_while_playing() {
    let log_lines = Arc::new(Mutex::new(VecDeque::new()));
    let batches = Arc::new(Mutex::new(vec![None, None, None]));
    let play_measure_mmls = Arc::new(Mutex::new(vec![
        "c".to_string(),
        "d".to_string(),
        "e".to_string(),
        "f".to_string(),
    ]));
    let (cache_tx, cache_rx) = std::sync::mpsc::channel();
    let cache = Arc::new(Mutex::new(vec![vec![super::CellCache::empty(); 5]; 3]));
    let completion_ctx = TrackRerenderBatchCompletionContext {
        batches: Arc::clone(&batches),
        log_lines: Arc::clone(&log_lines),
        cache: Arc::clone(&cache),
        play_position: Arc::new(Mutex::new(Some(super::PlayPosition {
            measure_index: 0,
            measure_start: std::time::Instant::now(),
            measure_duration: std::time::Duration::from_secs(1),
        }))),
        ab_repeat: Arc::new(Mutex::new(super::AbRepeatState::Off)),
        play_measure_mmls: Arc::clone(&play_measure_mmls),
        cache_tx,
        cache_render_workers: cmrt_runtime::DEFAULT_OFFLINE_RENDER_WORKERS,
    };
    {
        let mut cache_guard = cache.lock().unwrap();
        for measure in 2..=4 {
            cache_guard[2][measure].state = super::CacheState::Pending;
            cache_guard[2][measure].generation = 1;
        }
    }
    batches.lock().unwrap()[2] = Some(TrackRerenderBatch {
        pending: BTreeMap::from([
            (
                2,
                CacheJob {
                    track: 2,
                    measure: 2,
                    measure_samples: 4,
                    generation: 1,
                    rendered_mml_hash: 2,
                    mml: "d".to_string(),
                },
            ),
            (
                3,
                CacheJob {
                    track: 2,
                    measure: 3,
                    measure_samples: 4,
                    generation: 1,
                    rendered_mml_hash: 3,
                    mml: "e".to_string(),
                },
            ),
            (
                4,
                CacheJob {
                    track: 2,
                    measure: 4,
                    measure_samples: 4,
                    generation: 1,
                    rendered_mml_hash: 4,
                    mml: "f".to_string(),
                },
            ),
        ]),
        active_measures: BTreeSet::from([1]),
        completion_log: "cache: rerender done track1 meas 1〜4 (random patch update)".to_string(),
    });

    DawApp::complete_track_rerender_batch_measure(&completion_ctx, 2, 1);

    assert_eq!(cache_rx.try_recv().unwrap().measure, 2);
    assert_eq!(cache_rx.try_recv().unwrap().measure, 3);
    assert!(cache_rx.try_recv().is_err());
    let batch = batches.lock().unwrap();
    let current_batch = batch[2].as_ref().expect("batch should continue");
    assert_eq!(current_batch.active_measures, BTreeSet::from([2, 3]));
    assert!(current_batch.pending.contains_key(&4));
}

#[test]
fn complete_track_rerender_batch_respects_global_worker_limit_across_tracks() {
    let log_lines = Arc::new(Mutex::new(VecDeque::new()));
    let batches = Arc::new(Mutex::new(vec![None, None, None, None]));
    let play_measure_mmls = Arc::new(Mutex::new(vec![
        "c".to_string(),
        "d".to_string(),
        "e".to_string(),
        "f".to_string(),
    ]));
    let (cache_tx, cache_rx) = std::sync::mpsc::channel();
    let cache = Arc::new(Mutex::new(vec![vec![super::CellCache::empty(); 5]; 4]));
    let completion_ctx = TrackRerenderBatchCompletionContext {
        batches: Arc::clone(&batches),
        log_lines: Arc::clone(&log_lines),
        cache: Arc::clone(&cache),
        play_position: Arc::new(Mutex::new(None)),
        ab_repeat: Arc::new(Mutex::new(super::AbRepeatState::Off)),
        play_measure_mmls: Arc::clone(&play_measure_mmls),
        cache_tx,
        cache_render_workers: 4,
    };
    {
        let mut cache_guard = cache.lock().unwrap();
        cache_guard[2][2].state = super::CacheState::Pending;
        cache_guard[2][2].generation = 1;
        for measure in 2..=4 {
            cache_guard[3][measure].state = super::CacheState::Pending;
            cache_guard[3][measure].generation = 1;
        }
    }
    batches.lock().unwrap()[2] = Some(TrackRerenderBatch {
        pending: BTreeMap::from([(
            2,
            CacheJob {
                track: 2,
                measure: 2,
                measure_samples: 4,
                generation: 1,
                rendered_mml_hash: 2,
                mml: "d".to_string(),
            },
        )]),
        active_measures: BTreeSet::from([1]),
        completion_log: "cache: rerender done track1 meas 1〜2 (random patch update)".to_string(),
    });
    batches.lock().unwrap()[3] = Some(TrackRerenderBatch {
        pending: BTreeMap::from([(
            4,
            CacheJob {
                track: 3,
                measure: 4,
                measure_samples: 4,
                generation: 1,
                rendered_mml_hash: 14,
                mml: "f".to_string(),
            },
        )]),
        active_measures: BTreeSet::from([1, 2, 3]),
        completion_log: "cache: rerender done track2 meas 1〜4 (random patch update)".to_string(),
    });

    DawApp::complete_track_rerender_batch_measure(&completion_ctx, 2, 1);

    let queued_job = cache_rx
        .try_recv()
        .expect("only one global worker slot should be refilled");
    assert_eq!((queued_job.track, queued_job.measure), (2, 2));
    assert!(
        cache_rx.try_recv().is_err(),
        "global worker limit 4 では追加予約は 1 件だけ"
    );
    let batch = batches.lock().unwrap();
    assert_eq!(
        batch[2].as_ref().unwrap().active_measures,
        BTreeSet::from([2])
    );
    assert_eq!(
        batch[3].as_ref().unwrap().active_measures,
        BTreeSet::from([1, 2, 3])
    );
}
