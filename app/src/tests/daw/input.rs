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

fn build_test_app() -> (DawApp, std::sync::mpsc::Receiver<super::super::CacheJob>) {
    let tracks = 3;
    let measures = 2;
    let (cache_tx, cache_rx) = std::sync::mpsc::channel();
    (
        DawApp {
            data: vec![vec![String::new(); measures + 1]; tracks],
            cursor_track: 1,
            cursor_measure: 1,
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
                daw_tracks: tracks,
                daw_measures: measures,
            }),
            entry_ptr: 0,
            tracks,
            measures,
            cache: Arc::new(Mutex::new(vec![
                vec![CellCache::empty(); measures + 1];
                tracks
            ])),
            cache_tx,
            render_lock: Arc::new(Mutex::new(())),
            play_state: Arc::new(Mutex::new(DawPlayState::Idle)),
            play_transition_lock: Arc::new(Mutex::new(())),
            preview_session: Arc::new(std::sync::atomic::AtomicU64::new(0)),
            preview_sink: Arc::new(Mutex::new(None)),
            play_position: Arc::new(Mutex::new(None)),
            ab_repeat: Arc::new(Mutex::new(AbRepeatState::Off)),
            play_measure_mmls: Arc::new(Mutex::new(vec![String::new(); measures])),
            play_measure_track_mmls: Arc::new(Mutex::new(vec![
                vec![String::new(); tracks];
                measures
            ])),
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
        },
        cache_rx,
    )
}

#[path = "input/history.rs"]
mod history;
#[path = "input/insert.rs"]
mod insert;
#[path = "input/mixer.rs"]
mod mixer;
#[path = "input/normal.rs"]
mod normal;
