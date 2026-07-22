pub(super) use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

pub(super) use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
pub(super) use tui_textarea::{CursorMove, TextArea};

pub(super) use crate::config::Config;

pub(super) use super::super::{
    AbRepeatState, CacheState, CellCache, DawApp, DawHistoryPane, DawMode, DawNormalAction,
    DawPatchSelectPane, DawPlayState, PlayPosition,
};
pub(super) use super::{
    normal_playback_shortcut, preview_target_tracks, resolve_playback_start_measure_index,
    NormalPlaybackShortcut,
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

fn build_test_app() -> (DawApp, std::sync::mpsc::Receiver<super::super::CacheJob>) {
    let tracks = 3;
    let measures = 2;
    let (cache_tx, cache_rx) = std::sync::mpsc::channel();
    (
        DawApp {
            editor: crate::daw::editor::DawEditorState::new(
                vec![vec![String::new(); measures + 1]; tracks],
                1,
                1,
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
            playback: crate::daw::playback_runtime::DawPlaybackRuntime::for_test(tracks, measures),
            log_lines: Arc::new(Mutex::new(VecDeque::new())),
            track_rerender_batches: Arc::new(Mutex::new(vec![None; tracks])),
            solo_tracks: vec![false; tracks],
            track_volumes_db: vec![0; tracks],
            overlays: crate::daw::overlays::DawOverlays::new(1),
            patch_phrase_store: crate::history::PatchPhraseStore::default(),
            patch_phrase_store_dirty: false,

            random_patch_decks: crate::random::RandomIndexDecks::default(),
        },
        cache_rx,
    )
}

mod history_overlay;
mod insert;
mod mixer;
mod normal;
mod patch_select;
