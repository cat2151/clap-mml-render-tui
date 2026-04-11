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
fn start_track_rerender_batch_logs_only_targeted_measures() {
    use crate::config::Config;
    use crate::daw::{
        CacheState, CellCache, DawHistoryPane, DawMode, DawPatchSelectPane, DawPlayState,
    };
    use std::collections::VecDeque;
    use tui_textarea::TextArea;

    let tracks = 3;
    let measures = 4;
    let (cache_tx, cache_rx) = std::sync::mpsc::channel();
    let mut app = DawApp {
        data: vec![vec![String::new(); measures + 1]; tracks],
        cursor_track: 0,
        cursor_measure: 0,
        mode: DawMode::Normal,
        help_origin: DawMode::Normal,
        textarea: TextArea::default(),
        cfg: Arc::new(Config {
            plugin_path: String::new(),
            input_midi: String::new(),
            output_midi: String::new(),
            output_wav: String::new(),
            sample_rate: 44_100.0,
            buffer_size: 512,
            patches_dirs: None,
        }),
        entry_ptr: 0,
        tracks,
        measures,
        cache: Arc::new(Mutex::new(vec![
            vec![
                CellCache {
                    state: CacheState::Empty,
                    samples: None,
                    rendered_measure_samples: None,
                    generation: 0,
                    rendered_mml_hash: None,
                };
                measures + 1
            ];
            tracks
        ])),
        cache_tx,
        play_state: Arc::new(Mutex::new(DawPlayState::Idle)),
        play_transition_lock: Arc::new(Mutex::new(())),
        preview_session: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        preview_sink: Arc::new(Mutex::new(None)),
        play_position: Arc::new(Mutex::new(None)),
        ab_repeat: Arc::new(Mutex::new(super::AbRepeatState::Off)),
        overlay_preview_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
        play_measure_mmls: Arc::new(Mutex::new(vec![String::new(); measures])),
        play_measure_track_mmls: Arc::new(Mutex::new(vec![vec![String::new(); tracks]; measures])),
        play_measure_samples: Arc::new(Mutex::new(0)),
        log_lines: Arc::new(Mutex::new(VecDeque::new())),
        track_rerender_batches: Arc::new(Mutex::new(vec![None; tracks])),
        solo_tracks: vec![false; tracks],
        track_volumes_db: vec![0; tracks],
        mixer_cursor_track: 1,
        play_track_gains: Arc::new(Mutex::new(vec![0.0; tracks])),
        yank_buffer: None,
        normal_pending_delete: false,
        patch_phrase_store: crate::history::PatchPhraseStore::default(),
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
    app.data[1][1] = "c".to_string();
    app.data[1][3] = "e".to_string();
    app.data[1][4] = "g".to_string();
    {
        let mut cache = app.cache.lock().unwrap();
        cache[1][1].state = super::CacheState::Pending;
        cache[1][3].state = super::CacheState::Pending;
        cache[1][4].state = super::CacheState::Pending;
    }

    app.start_track_rerender_batch(1, &[1, 3, 4], "random patch update");

    let logs = app.log_lines.lock().unwrap().clone();
    assert!(
        logs.iter()
            .any(|line| line
                == "cache: rerender start track1 meas 1, meas 3〜4 (random patch update)")
    );
    assert!(logs
        .iter()
        .any(|line| line == "cache: rerender reserve track1 meas1 (meas1 -> meas3 -> meas4)"));
    assert!(logs
        .iter()
        .any(|line| line == "cache: rerender reserve track1 meas3 (meas3 -> meas4)"));
    assert!(logs
        .iter()
        .any(|line| line == "cache: rerender reserve track1 meas4 (meas4)"));
    assert_eq!(cache_rx.try_recv().unwrap().measure, 1);
    assert_eq!(cache_rx.try_recv().unwrap().measure, 3);
    assert_eq!(cache_rx.try_recv().unwrap().measure, 4);
    assert!(cache_rx.try_recv().is_err());
}
