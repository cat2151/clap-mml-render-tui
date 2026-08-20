use ratatui_textarea::TextArea;

use super::*;
use crate::{CacheState, CellCache, DawMode};
use cmrt_runtime::Config;

#[test]
fn start_track_rerender_batch_logs_only_targeted_measures() {
    let tracks = 3;
    let measures = 4;
    let cache_render_workers = 4;
    let (cache_tx, cache_rx) = std::sync::mpsc::channel();
    let mut app = DawApp {
        editor: crate::editor::DawEditorState::new(
            vec![vec![String::new(); measures + 1]; tracks],
            0,
            0,
            tracks,
            measures,
        ),
        mode: DawMode::Normal,
        help_origin: DawMode::Normal,
        sound_check_guide: cmrt_tui_core::sound_check_guide::SoundCheckGuide::new(None),
        textarea: TextArea::default(),
        cfg: Arc::new(Config {
            plugin_path: String::new(),
            input_midi: String::new(),
            output_midi: String::new(),
            output_wav: String::new(),
            sample_rate: 44_100.0,
            buffer_size: 512,
            patches_dirs: None,
            loop_dirs: Vec::new(),
            loop_categories: cmrt_runtime::default_loop_categories(),
            offline_render_workers: cache_render_workers,
            offline_render_server_workers: cmrt_runtime::DEFAULT_OFFLINE_RENDER_SERVER_WORKERS,
            offline_render_backend: cmrt_runtime::OfflineRenderBackend::InProcess,
            offline_render_server_port: cmrt_runtime::DEFAULT_OFFLINE_RENDER_SERVER_PORT,
            offline_render_server_command: String::new(),
            realtime_audio_backend: cmrt_runtime::RealtimeAudioBackend::InProcess,
            realtime_play_server_port: cmrt_runtime::DEFAULT_REALTIME_PLAY_SERVER_PORT,
            realtime_play_server_command: String::new(),
            realtime_play_server_prewarm: false,
            autoplay_on_startup: true,
            voicing_shared_source: String::new(),
            voicing_override_source: String::new(),
            chord_progression_source: String::new(),
            ..Default::default()
        }),
        plugin_entries: cmrt_offline_render::PluginEntries::none(),
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
        cache_render_workers,
        render_queue: crate::render_queue::RenderQueue::disabled_for_tests(),
        playback: crate::playback_runtime::DawPlaybackRuntime::for_test(tracks, measures),
        log_lines: Arc::new(Mutex::new(VecDeque::new())),
        track_rerender_batches: Arc::new(Mutex::new(vec![None; tracks])),
        solo_tracks: vec![false; tracks],
        track_volumes_db: vec![0; tracks],
        overlays: crate::overlays::DawOverlays::new(1),
        patch_phrase_store: cmrt_history::PatchPhraseStore::default(),
        patch_phrase_store_dirty: false,

        random_patch_decks: cmrt_tui_core::random::RandomIndexDecks::default(),
    };
    app.editor.data[1][1] = "c".to_string();
    app.editor.data[1][3] = "e".to_string();
    app.editor.data[1][4] = "g".to_string();
    {
        let mut cache = app.cache.lock().unwrap();
        cache[1][1].state = CacheState::Pending;
        cache[1][3].state = CacheState::Pending;
        cache[1][4].state = CacheState::Pending;
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
