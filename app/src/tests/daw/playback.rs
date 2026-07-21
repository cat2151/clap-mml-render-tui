pub(super) use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub(super) use tui_textarea::TextArea;

pub(super) use crate::config::Config;

pub(super) use super::{
    super::{AbRepeatState, CacheState, CellCache, DawApp, DawMode, DawPlayState},
    cache_mixer::{
        build_playback_measure_samples, pad_playback_measure_samples, try_get_cached_samples,
        PlaybackMeasureRequest,
    },
    measure_math::{
        current_play_measure_index, following_measure_index, format_playback_future_append_log,
        format_playback_measure_advance_log, format_playback_measure_resolution_log,
        future_chunk_append_deadline, resolved_measure_start_after_append,
    },
    measure_mixer::{mix_measure_chunk, ActiveMeasureLayer},
    wait_until_or_stop,
};

/// stop_play のログ出力を検証するための最小構成の DawApp を作る。
fn build_test_app() -> DawApp {
    let tracks = 3;
    let measures = 2;
    let (cache_tx, _cache_rx) = std::sync::mpsc::channel();
    DawApp {
        editor: crate::daw::editor::DawEditorState::new(
            vec![vec![String::new(); measures + 1]; tracks],
            0,
            0,
            tracks,
            measures,
        ),
        mode: DawMode::Normal,
        help_origin: DawMode::Normal,
        sound_check_guide: crate::sound_check_guide::SoundCheckGuide::new(None),
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
            loop_categories: crate::config::default_loop_categories(),
            offline_render_workers: crate::config::DEFAULT_OFFLINE_RENDER_WORKERS,
            offline_render_server_workers: crate::config::DEFAULT_OFFLINE_RENDER_SERVER_WORKERS,
            offline_render_backend: crate::config::OfflineRenderBackend::InProcess,
            offline_render_server_port: crate::config::DEFAULT_OFFLINE_RENDER_SERVER_PORT,
            offline_render_server_command: String::new(),
            realtime_audio_backend: crate::config::RealtimeAudioBackend::InProcess,
            realtime_play_server_port: crate::config::DEFAULT_REALTIME_PLAY_SERVER_PORT,
            realtime_play_server_command: String::new(),
            autoplay_on_startup: true,
            voicing_shared_source: String::new(),
            voicing_override_source: String::new(),
        }),
        entry_ptr: 0,
        cache: Arc::new(Mutex::new(vec![
            vec![CellCache::empty(); measures + 1];
            tracks
        ])),
        cache_tx,
        cache_render_workers: crate::config::DEFAULT_OFFLINE_RENDER_WORKERS,
        render_queue: crate::daw::render_queue::RenderQueue::disabled_for_tests(),
        play_state: Arc::new(Mutex::new(DawPlayState::Idle)),
        play_transition_lock: Arc::new(Mutex::new(())),
        preview_session: Arc::new(std::sync::atomic::AtomicU64::new(0)),
        preview_sink: Arc::new(Mutex::new(None)),
        realtime_play_server: None,
        play_position: Arc::new(Mutex::new(None)),
        ab_repeat: Arc::new(Mutex::new(AbRepeatState::Off)),
        overlay_preview_cache: Arc::new(Mutex::new(std::collections::HashMap::new())),
        play_measure_mmls: Arc::new(Mutex::new(vec![String::new(); measures])),
        play_measure_track_mmls: Arc::new(Mutex::new(vec![vec![String::new(); tracks]; measures])),
        play_measure_samples: Arc::new(Mutex::new(0)),
        log_lines: Arc::new(Mutex::new(VecDeque::new())),
        track_rerender_batches: Arc::new(Mutex::new(vec![None; tracks])),
        solo_tracks: vec![false; tracks],
        track_volumes_db: vec![0; tracks],
        overlays: crate::daw::overlays::DawOverlays::new(1),
        play_track_gains: Arc::new(Mutex::new(vec![0.0; tracks])),
        patch_phrase_store: crate::history::PatchPhraseStore::default(),
        patch_phrase_store_dirty: false,

        random_patch_decks: crate::random::RandomIndexDecks::default(),
    }
}

fn playback_track_mmls(track: usize, mml: &str) -> Vec<String> {
    let mut track_mmls = vec![String::new(); 3];
    track_mmls[track] = mml.to_string();
    track_mmls
}

fn playback_track_gains() -> Vec<f32> {
    vec![0.0, 1.0, 1.0]
}

#[path = "playback/cache_mixer.rs"]
mod cache_mixer;
#[path = "playback/state.rs"]
mod state;
#[path = "playback/timing.rs"]
mod timing;
