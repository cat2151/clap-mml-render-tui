use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{Arc, Mutex},
};

use super::wav_io::load_wav_samples;
use super::{
    batch_logging::{TrackRerenderBatch, TrackRerenderBatchCompletionContext},
    CacheJob, DawApp,
};

#[test]
fn load_wav_samples_reads_back_float_wav_cache() {
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_daw_cache_{}_{}.wav",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 44_100,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    {
        let mut writer = hound::WavWriter::create(&tmp, spec).unwrap();
        writer.write_sample(0.25f32).unwrap();
        writer.write_sample(-0.25f32).unwrap();
        writer.finalize().unwrap();
    }

    let samples = load_wav_samples(&tmp).unwrap();
    std::fs::remove_file(&tmp).ok();

    assert_eq!(samples, vec![0.25, -0.25]);
}

#[test]
fn complete_track_rerender_batch_logs_only_after_last_measure_finishes() {
    let log_lines = Arc::new(Mutex::new(VecDeque::new()));
    let batches = Arc::new(Mutex::new(vec![None, None]));
    let play_position = Arc::new(Mutex::new(None));
    let play_measure_mmls = Arc::new(Mutex::new(vec!["c".to_string(), "d".to_string()]));
    let (cache_tx, cache_rx) = std::sync::mpsc::channel();
    let cache = Arc::new(Mutex::new(vec![vec![super::CellCache::empty(); 3]; 2]));
    let completion_ctx = TrackRerenderBatchCompletionContext {
        batches: Arc::clone(&batches),
        log_lines: Arc::clone(&log_lines),
        cache: Arc::clone(&cache),
        play_position: Arc::clone(&play_position),
        ab_repeat: Arc::new(Mutex::new(super::AbRepeatState::Off)),
        play_measure_mmls: Arc::clone(&play_measure_mmls),
        cache_tx,
        cache_render_workers: crate::config::DEFAULT_OFFLINE_RENDER_WORKERS,
    };
    {
        let mut cache_guard = cache.lock().unwrap();
        cache_guard[1][2].state = super::CacheState::Pending;
        cache_guard[1][2].generation = 1;
    }
    batches.lock().unwrap()[1] = Some(TrackRerenderBatch {
        pending: BTreeMap::from([(
            2,
            CacheJob {
                track: 1,
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

    DawApp::complete_track_rerender_batch_measure(&completion_ctx, 1, 1);
    assert!(
        log_lines
            .lock()
            .unwrap()
            .iter()
            .any(|line| line == "cache: rerender reserve track1 meas2 (meas2)"),
        "next reservation log should be emitted before completion"
    );
    assert_eq!(cache_rx.try_recv().unwrap().measure, 2);
    assert!(!log_lines
        .lock()
        .unwrap()
        .iter()
        .any(|line| line == "cache: rerender done track1 meas 1〜2 (random patch update)"));

    DawApp::complete_track_rerender_batch_measure(&completion_ctx, 1, 2);

    assert_eq!(
        log_lines.lock().unwrap().back().map(String::as_str),
        Some("cache: rerender done track1 meas 1〜2 (random patch update)")
    );
    assert!(
        batches.lock().unwrap()[1].is_none(),
        "completed batch should be cleared"
    );
}

#[test]
fn complete_track_rerender_batch_waits_for_last_active_measure_before_logging_done() {
    let log_lines = Arc::new(Mutex::new(VecDeque::new()));
    let batches = Arc::new(Mutex::new(vec![None, None]));
    let completion_log = "cache: rerender done track1 meas 1〜2 (random patch update)";
    let (cache_tx, _cache_rx) = std::sync::mpsc::channel();
    let completion_ctx = TrackRerenderBatchCompletionContext {
        batches: Arc::clone(&batches),
        log_lines: Arc::clone(&log_lines),
        cache: Arc::new(Mutex::new(vec![vec![super::CellCache::empty(); 3]; 2])),
        play_position: Arc::new(Mutex::new(None)),
        ab_repeat: Arc::new(Mutex::new(super::AbRepeatState::Off)),
        play_measure_mmls: Arc::new(Mutex::new(vec!["c".to_string(), "d".to_string()])),
        cache_tx,
        cache_render_workers: crate::config::DEFAULT_OFFLINE_RENDER_WORKERS,
    };
    batches.lock().unwrap()[1] = Some(TrackRerenderBatch {
        pending: BTreeMap::new(),
        active_measures: BTreeSet::from([1, 2]),
        completion_log: completion_log.to_string(),
    });

    DawApp::complete_track_rerender_batch_measure(&completion_ctx, 1, 1);

    assert!(
        batches.lock().unwrap()[1].is_some(),
        "batch should remain active while another measure is still rendering"
    );
    assert!(!log_lines
        .lock()
        .unwrap()
        .iter()
        .any(|line| line == completion_log));

    DawApp::complete_track_rerender_batch_measure(&completion_ctx, 1, 2);

    assert!(batches.lock().unwrap()[1].is_none());
    assert_eq!(
        log_lines.lock().unwrap().back().map(String::as_str),
        Some(completion_log)
    );
}

#[test]
fn complete_track_rerender_batch_skips_stale_pending_job_and_reserves_next_measure() {
    let log_lines = Arc::new(Mutex::new(VecDeque::new()));
    let batches = Arc::new(Mutex::new(vec![None, None]));
    let play_position = Arc::new(Mutex::new(None));
    let play_measure_mmls = Arc::new(Mutex::new(vec![
        "c".to_string(),
        "d".to_string(),
        "e".to_string(),
    ]));
    let (cache_tx, cache_rx) = std::sync::mpsc::channel();
    let cache = Arc::new(Mutex::new(vec![vec![super::CellCache::empty(); 4]; 2]));
    let completion_ctx = TrackRerenderBatchCompletionContext {
        batches: Arc::clone(&batches),
        log_lines: Arc::clone(&log_lines),
        cache: Arc::clone(&cache),
        play_position: Arc::clone(&play_position),
        ab_repeat: Arc::new(Mutex::new(super::AbRepeatState::Off)),
        play_measure_mmls: Arc::clone(&play_measure_mmls),
        cache_tx,
        cache_render_workers: crate::config::DEFAULT_OFFLINE_RENDER_WORKERS,
    };
    {
        let mut cache_guard = cache.lock().unwrap();
        cache_guard[1][2].state = super::CacheState::Pending;
        cache_guard[1][2].generation = 2;
        cache_guard[1][3].state = super::CacheState::Pending;
        cache_guard[1][3].generation = 1;
    }
    batches.lock().unwrap()[1] = Some(TrackRerenderBatch {
        pending: BTreeMap::from([
            (
                2,
                CacheJob {
                    track: 1,
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
                    track: 1,
                    measure: 3,
                    measure_samples: 4,
                    generation: 1,
                    rendered_mml_hash: 3,
                    mml: "e".to_string(),
                },
            ),
        ]),
        active_measures: BTreeSet::from([1]),
        completion_log: "cache: rerender done track1 meas 1〜3 (random patch update)".to_string(),
    });

    DawApp::complete_track_rerender_batch_measure(&completion_ctx, 1, 1);

    let next_job = cache_rx
        .try_recv()
        .expect("next valid measure should be reserved");
    assert_eq!(next_job.measure, 3);
    let logs = log_lines.lock().unwrap().clone();
    assert!(
        logs.iter()
            .any(|line| line == "cache: rerender reserve track1 meas3 (meas3)"),
        "logs: {:?}",
        logs
    );
    let batch = batches.lock().unwrap();
    let current_batch = batch[1].as_ref().expect("batch should continue");
    assert_eq!(current_batch.active_measures, BTreeSet::from([3]));
    assert!(
        !current_batch.pending.contains_key(&2),
        "stale pending measure should be dropped"
    );
}

#[test]
fn complete_track_rerender_batch_respects_cache_render_worker_limit() {
    let log_lines = Arc::new(Mutex::new(VecDeque::new()));
    let batches = Arc::new(Mutex::new(vec![None, None]));
    let play_measure_mmls = Arc::new(Mutex::new(vec![
        "c".to_string(),
        "d".to_string(),
        "e".to_string(),
        "f".to_string(),
    ]));
    let (cache_tx, cache_rx) = std::sync::mpsc::channel();
    let cache = Arc::new(Mutex::new(vec![vec![super::CellCache::empty(); 5]; 2]));
    let completion_ctx = TrackRerenderBatchCompletionContext {
        batches: Arc::clone(&batches),
        log_lines: Arc::clone(&log_lines),
        cache: Arc::clone(&cache),
        play_position: Arc::new(Mutex::new(None)),
        ab_repeat: Arc::new(Mutex::new(super::AbRepeatState::Off)),
        play_measure_mmls: Arc::clone(&play_measure_mmls),
        cache_tx,
        cache_render_workers: 2,
    };
    {
        let mut cache_guard = cache.lock().unwrap();
        for measure in 3..=4 {
            cache_guard[1][measure].state = super::CacheState::Pending;
            cache_guard[1][measure].generation = 1;
        }
    }
    batches.lock().unwrap()[1] = Some(TrackRerenderBatch {
        pending: BTreeMap::from([
            (
                3,
                CacheJob {
                    track: 1,
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
                    track: 1,
                    measure: 4,
                    measure_samples: 4,
                    generation: 1,
                    rendered_mml_hash: 4,
                    mml: "f".to_string(),
                },
            ),
        ]),
        active_measures: BTreeSet::from([1, 2]),
        completion_log: "cache: rerender done track1 meas 1〜4 (random patch update)".to_string(),
    });

    DawApp::complete_track_rerender_batch_measure(&completion_ctx, 1, 1);

    assert_eq!(cache_rx.try_recv().unwrap().measure, 3);
    assert!(
        cache_rx.try_recv().is_err(),
        "worker limit 2 では一度に追加予約する小節は 1 つだけ"
    );
    let batch = batches.lock().unwrap();
    let current_batch = batch[1].as_ref().expect("batch should continue");
    assert_eq!(current_batch.active_measures, BTreeSet::from([2, 3]));
    assert!(current_batch.pending.contains_key(&4));
}

#[test]
fn complete_track_rerender_batch_uses_available_worker_slots_while_playing() {
    let log_lines = Arc::new(Mutex::new(VecDeque::new()));
    let batches = Arc::new(Mutex::new(vec![None, None]));
    let play_measure_mmls = Arc::new(Mutex::new(vec![
        "c".to_string(),
        "d".to_string(),
        "e".to_string(),
        "f".to_string(),
    ]));
    let (cache_tx, cache_rx) = std::sync::mpsc::channel();
    let cache = Arc::new(Mutex::new(vec![vec![super::CellCache::empty(); 5]; 2]));
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
        cache_render_workers: crate::config::DEFAULT_OFFLINE_RENDER_WORKERS,
    };
    {
        let mut cache_guard = cache.lock().unwrap();
        for measure in 2..=4 {
            cache_guard[1][measure].state = super::CacheState::Pending;
            cache_guard[1][measure].generation = 1;
        }
    }
    batches.lock().unwrap()[1] = Some(TrackRerenderBatch {
        pending: BTreeMap::from([
            (
                2,
                CacheJob {
                    track: 1,
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
                    track: 1,
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
                    track: 1,
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

    DawApp::complete_track_rerender_batch_measure(&completion_ctx, 1, 1);

    assert_eq!(cache_rx.try_recv().unwrap().measure, 2);
    assert_eq!(cache_rx.try_recv().unwrap().measure, 3);
    assert!(cache_rx.try_recv().is_err());
    let batch = batches.lock().unwrap();
    let current_batch = batch[1].as_ref().expect("batch should continue");
    assert_eq!(current_batch.active_measures, BTreeSet::from([2, 3]));
    assert!(current_batch.pending.contains_key(&4));
}

#[test]
fn complete_track_rerender_batch_respects_global_worker_limit_across_tracks() {
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
        play_position: Arc::new(Mutex::new(None)),
        ab_repeat: Arc::new(Mutex::new(super::AbRepeatState::Off)),
        play_measure_mmls: Arc::clone(&play_measure_mmls),
        cache_tx,
        cache_render_workers: 4,
    };
    {
        let mut cache_guard = cache.lock().unwrap();
        cache_guard[1][2].state = super::CacheState::Pending;
        cache_guard[1][2].generation = 1;
        for measure in 2..=4 {
            cache_guard[2][measure].state = super::CacheState::Pending;
            cache_guard[2][measure].generation = 1;
        }
    }
    batches.lock().unwrap()[1] = Some(TrackRerenderBatch {
        pending: BTreeMap::from([(
            2,
            CacheJob {
                track: 1,
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
    batches.lock().unwrap()[2] = Some(TrackRerenderBatch {
        pending: BTreeMap::from([(
            4,
            CacheJob {
                track: 2,
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

    DawApp::complete_track_rerender_batch_measure(&completion_ctx, 1, 1);

    let queued_job = cache_rx
        .try_recv()
        .expect("only one global worker slot should be refilled");
    assert_eq!((queued_job.track, queued_job.measure), (1, 2));
    assert!(
        cache_rx.try_recv().is_err(),
        "global worker limit 4 では追加予約は 1 件だけ"
    );
    let batch = batches.lock().unwrap();
    assert_eq!(
        batch[1].as_ref().unwrap().active_measures,
        BTreeSet::from([2])
    );
    assert_eq!(
        batch[2].as_ref().unwrap().active_measures,
        BTreeSet::from([1, 2, 3])
    );
}

#[path = "mod/start_track_rerender_batch.rs"]
mod start_track_rerender_batch;
