pub(super) use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub(super) use ratatui_textarea::TextArea;

pub(super) use cmrt_runtime::Config;

pub(super) use super::{
    super::{CellCache, DawApp, DawMode, DawPlayState},
    measure_math::{
        current_play_measure_index, following_measure_index, format_playback_measure_advance_log,
        format_playback_measure_resolution_log,
    },
    wait_until_or_stop,
};

/// stop_play のログ出力を検証するための最小構成の DawApp を作る。
fn build_test_app() -> DawApp {
    // 0 = Tempo / 1 = chord 行 / 2..=3 = 演奏 track。
    let tracks = crate::FIRST_PLAYABLE_TRACK + 2;
    let measures = 2;
    let (cache_tx, _cache_rx) = std::sync::mpsc::channel();
    DawApp {
        workspace_kind: crate::WorkspaceKind::Persistent,
        daily_page_date: None,
        config_app_dir: None,
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
            offline_render_workers: cmrt_runtime::DEFAULT_OFFLINE_RENDER_WORKERS,
            offline_render_server_workers: cmrt_runtime::DEFAULT_OFFLINE_RENDER_SERVER_WORKERS,
            offline_render_backend: cmrt_runtime::OfflineRenderBackend::InProcess,
            offline_render_server_port: cmrt_runtime::DEFAULT_OFFLINE_RENDER_SERVER_PORT,
            offline_render_server_command: String::new(),
            realtime_audio_backend: cmrt_runtime::RealtimeAudioBackend::CachePlayer,
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
            vec![CellCache::empty(); measures + 1];
            tracks
        ])),
        cache_tx,
        cache_render_workers: cmrt_runtime::DEFAULT_OFFLINE_RENDER_WORKERS,
        render_queue: crate::render_queue::RenderQueue::disabled_for_tests(),
        playback: crate::playback_runtime::DawPlaybackRuntime::for_test(tracks, measures),
        log_lines: Arc::new(Mutex::new(VecDeque::new())),
        track_rerender_batches: Arc::new(Mutex::new(vec![None; tracks])),
        solo_tracks: vec![false; tracks],
        track_volumes_db: vec![0; tracks],
        pending_auto_trim: false,
        overlays: crate::overlays::DawOverlays::new(crate::FIRST_PLAYABLE_TRACK),
        patch_phrase_store: cmrt_history::PatchPhraseStore::default(),
        patch_phrase_store_dirty: false,

        random_patch_decks: cmrt_tui_core::random::RandomIndexDecks::default(),
        chord_progression_source: None,
        patch_load: Arc::new(Mutex::new(
            cmrt_tui_core::patch_load::PatchLoadState::Loading,
        )),
        mml_overlay: cmrt_mml_overlay::MmlOverlay::default(),
        mml_overlay_sender: None,
    }
}

mod state;
mod timing;
