pub(super) use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

pub(super) use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
pub(super) use ratatui_textarea::{CursorMove, TextArea};

pub(super) use cmrt_runtime::Config;

pub(super) use super::super::{
    AbRepeatState, CacheState, CellCache, DawApp, DawHistoryPane, DawMode, DawNormalAction,
    DawPatchSelectPane, DawPlayState, DawProjectFileAction, PlayPosition,
};
pub(super) use super::{
    cursor_move_preview_track, normal_playback_shortcut, preview_target_tracks,
    resolve_playback_start_measure_index, NormalPlaybackShortcut,
};

/// -6dB を線形 gain 値に変換する（10^(-6/20)）。
fn track1_minus_6_db_gain() -> f32 {
    10.0f32.powf(-6.0 / 20.0)
}

struct TempDirGuard(std::path::PathBuf);

impl TempDirGuard {
    fn new(name: &str) -> Self {
        let path = std::env::temp_dir().join(name);
        std::fs::remove_dir_all(&path).ok();
        Self(path)
    }

    fn path(&self) -> &std::path::Path {
        &self.0
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        std::fs::remove_dir_all(&self.0).ok();
    }
}

pub(crate) fn build_test_app() -> (DawApp, std::sync::mpsc::Receiver<super::super::CacheJob>) {
    // 0 = Tempo / 1 = chord 行 / 2..=3 = 演奏 track。
    let tracks = crate::FIRST_PLAYABLE_TRACK + 2;
    let measures = 2;
    let (cache_tx, cache_rx) = std::sync::mpsc::channel();
    (
        DawApp {
            workspace_kind: crate::WorkspaceKind::Persistent,
            daily_page_date: None,
            config_app_dir: None,
            editor: crate::editor::DawEditorState::new(
                vec![vec![String::new(); measures + 1]; tracks],
                crate::FIRST_PLAYABLE_TRACK,
                1,
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
            overlays: crate::overlays::DawOverlays::new(1),
            patch_phrase_store: cmrt_history::PatchPhraseStore::default(),
            patch_phrase_store_dirty: false,

            random_patch_decks: cmrt_tui_core::random::RandomIndexDecks::default(),
            chord_progression_source: None,
            patch_load: Arc::new(Mutex::new(
                cmrt_tui_core::patch_load::PatchLoadState::Loading,
            )),
            mml_overlay: cmrt_mml_overlay::MmlOverlay::default(),
            mml_overlay_sender: None,
        },
        cache_rx,
    )
}

mod daily_project;
mod history_overlay;
mod insert;
mod mixer;
mod normal;
mod patch_select;
mod project;
