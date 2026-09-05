pub(super) use super::super::{DEFAULT_TRACK0_MML, MEASURES, TRACKS};
pub(super) use super::{
    apply_save_file_to_data, apply_save_file_to_track_volumes, data_to_save_file,
    load_saved_grid_size, required_grid_size, DawSaveFile,
};
pub(super) use crate::{CellCache, DawApp, DawMode};
pub(super) use ratatui_textarea::TextArea;
pub(super) use std::collections::VecDeque;
pub(super) use std::sync::{Arc, Mutex};

/// テスト用ヘルパー: TRACKS×(MEASURES+1) の空 data を作成する
pub(super) fn empty_data(tracks: usize, measures: usize) -> Vec<Vec<String>> {
    vec![vec![String::new(); measures + 1]; tracks]
}

pub(super) fn empty_track_volumes(tracks: usize) -> Vec<i32> {
    vec![0; tracks]
}

fn build_test_app(tracks: usize, measures: usize) -> DawApp {
    let (cache_tx, _cache_rx) = std::sync::mpsc::channel();
    DawApp {
        workspace_kind: crate::WorkspaceKind::Persistent,
        daily_page_date: None,
        config_app_dir: None,
        editor: crate::editor::DawEditorState::new(
            vec![vec![String::new(); measures + 1]; tracks],
            1.min(tracks - 1),
            1.min(measures),
            tracks,
            measures,
        ),
        mode: DawMode::Normal,
        help_origin: DawMode::Normal,
        sound_check_guide: cmrt_tui_core::sound_check_guide::SoundCheckGuide::new(None),
        textarea: TextArea::default(),
        cfg: Arc::new(cmrt_runtime::Config {
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
        overlays: crate::overlays::DawOverlays::new(1.min(tracks - 1)),
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

// ─── ensure_cmrt_dir ──────────────────────────────────────────

#[test]
fn ensure_cmrt_dir_is_idempotent() {
    // 複数回呼んでもエラーにならない（一時ディレクトリを使って設定ディレクトリを汚染しない）
    let tmp = std::env::temp_dir().join("cmrt_test_daw_idempotent");
    let _env_guard = cmrt_history::test_support::set_local_dir_envs(&tmp);
    std::fs::remove_dir_all(&tmp).ok();

    let r1 = cmrt_core::ensure_cmrt_dir();
    let r2 = cmrt_core::ensure_cmrt_dir();

    assert!(r1.is_ok(), "初回 ensure_cmrt_dir が失敗: {:?}", r1.err());
    assert!(r2.is_ok(), "2回目 ensure_cmrt_dir が失敗: {:?}", r2.err());

    drop(_env_guard);
    std::fs::remove_dir_all(&tmp).ok();
}

#[test]
fn save_history_state_flushes_dirty_patch_phrase_store() {
    let tmp = std::env::temp_dir().join("cmrt_test_daw_flush_patch_store");
    std::fs::remove_dir_all(&tmp).ok();
    std::fs::create_dir_all(&tmp).unwrap();
    let _guard = cmrt_history::test_support::set_local_dir_envs(&tmp);

    let mut app = build_test_app(3, 2);
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        cmrt_history::PatchPhraseState {
            history: vec!["cdef".to_string()],
            favorites: vec![],
        },
    );
    app.patch_phrase_store_dirty = true;

    app.save_history_state();

    let loaded = cmrt_history::load_patch_phrase_store();
    assert_eq!(
        loaded
            .patches
            .get("Pads/Pad 1.fxp")
            .map(|state| state.history.clone()),
        Some(vec!["cdef".to_string()])
    );

    std::fs::remove_dir_all(&tmp).ok();
}

mod chord_track;
mod json_format;
