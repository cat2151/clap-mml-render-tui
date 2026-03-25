use std::{
    collections::{BTreeSet, VecDeque},
    sync::{Arc, Mutex},
};

use super::wav_io::load_wav_samples;
use super::{batch_logging::TrackRerenderBatch, DawApp};

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
    batches.lock().unwrap()[1] = Some(TrackRerenderBatch {
        pending: BTreeSet::from([1, 2]),
        completion_log: "cache: rerender done track1 meas 1〜2 (random patch update)".to_string(),
    });

    DawApp::complete_track_rerender_batch_measure(&batches, &log_lines, 1, 1);
    assert!(
        log_lines.lock().unwrap().is_empty(),
        "completion log should wait for the last measure"
    );

    DawApp::complete_track_rerender_batch_measure(&batches, &log_lines, 1, 2);

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
fn start_track_rerender_batch_logs_only_targeted_measures() {
    use crate::config::Config;
    use crate::daw::{CacheState, CellCache, DawMode, DawPlayState};
    use std::collections::VecDeque;
    use tui_textarea::TextArea;

    let tracks = 3;
    let measures = 4;
    let (cache_tx, _cache_rx) = std::sync::mpsc::channel();
    let app = DawApp {
        data: vec![vec![String::new(); measures + 1]; tracks],
        cursor_track: 0,
        cursor_measure: 0,
        mode: DawMode::Normal,
        textarea: TextArea::default(),
        cfg: Arc::new(Config {
            plugin_path: String::new(),
            input_midi: String::new(),
            output_midi: String::new(),
            output_wav: String::new(),
            sample_rate: 44_100.0,
            buffer_size: 512,
            patch_path: None,
            patches_dir: None,
            daw_tracks: tracks,
            daw_measures: measures,
        }),
        entry_ptr: 0,
        tracks,
        measures,
        cache: Arc::new(Mutex::new(vec![
            vec![
                CellCache {
                    state: CacheState::Empty,
                    samples: None,
                    generation: 0,
                    rendered_mml_hash: None,
                };
                measures + 1
            ];
            tracks
        ])),
        cache_tx,
        render_lock: Arc::new(Mutex::new(())),
        play_state: Arc::new(Mutex::new(DawPlayState::Idle)),
        play_transition_lock: Arc::new(Mutex::new(())),
        play_position: Arc::new(Mutex::new(None)),
        play_measure_mmls: Arc::new(Mutex::new(vec![String::new(); measures])),
        play_measure_samples: Arc::new(Mutex::new(0)),
        log_lines: Arc::new(Mutex::new(VecDeque::new())),
        track_rerender_batches: Arc::new(Mutex::new(vec![None; tracks])),
    };

    app.start_track_rerender_batch(1, &[1, 3, 4], "random patch update");

    assert_eq!(
        app.log_lines.lock().unwrap().back().map(String::as_str),
        Some("cache: rerender start track1 meas 1, meas 3〜4 (random patch update)")
    );
}
