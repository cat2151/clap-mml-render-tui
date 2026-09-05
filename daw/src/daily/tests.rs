//! Daily ワークスペースのテスト。
//!
//! ここには**共有のヘルパだけ**を置き、テスト本体は責務ごとに
//! `tests/` 直下のサブモジュールへ分ける。

use std::{
    path::{Path, PathBuf},
    sync::mpsc::Receiver,
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value};

use super::*;
use crate::{CacheJob, CellCache, DawMode, WorkspaceKind, DEFAULT_TRACK0_MML, MEASURES, TRACKS};
use ratatui_textarea::TextArea;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

mod stale_cache;
mod wire_format;
mod workspace_entry;

pub(super) struct TempDirectory(PathBuf);

impl TempDirectory {
    pub(super) fn new(label: &str) -> Self {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "cmrt-daw-daily-{label}-{}-{nonce}",
            std::process::id()
        ));
        std::fs::create_dir_all(&path).unwrap();
        Self(path)
    }

    pub(super) fn path(&self) -> &Path {
        &self.0
    }
}

impl Drop for TempDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn project_value(mml: &str) -> Value {
    json!({
        "format": "clap-mml-render-tui.daw-project",
        "format_version": 1,
        "project": {
            "track_count": 2,
            "playable_measure_count": 2,
            "tracks": [
                {
                    "track_index": 0,
                    "role": "global_header",
                    "volume_db": 0,
                    "non_empty_cells": [{
                        "measure_index": 0,
                        "role": "initialization",
                        "mml": "{\"beat\":\"4/4\"}t120"
                    }]
                },
                {
                    "track_index": 1,
                    "role": "instrument",
                    "volume_db": -6,
                    "non_empty_cells": [{
                        "measure_index": 1,
                        "role": "playable_measure",
                        "mml": mml
                    }]
                }
            ]
        }
    })
}

fn recovery_value(page_date: &str, mml: &str) -> Value {
    json!({
        "page_date": page_date,
        "project_file": project_value(mml),
        "cursor_track": 1,
        "cursor_measure": 2,
        "cached_measures": [{
            "track": 1,
            "measure": 1,
            "mml_hash": 42
        }]
    })
}

fn recovery(page_date: &str, mml: &str) -> DailyRecoveryFile {
    decode_daily_recovery(&serde_json::to_string(&recovery_value(page_date, mml)).unwrap()).unwrap()
}

/// `cache_tx` の受け口ごと返す版。
///
/// `kick_all_pending()` が本当に 1 件も投入しないことを見るテストは、
/// レシーバを生かしておかないと「落ちた先が無いから空」になってしまう。
fn build_daily_app_with_cache_jobs(config_app_dir: &Path) -> (DawApp, Receiver<CacheJob>) {
    let mut data = vec![vec![String::new(); MEASURES + 1]; TRACKS];
    data[0][0] = DEFAULT_TRACK0_MML.to_string();
    let (cache_tx, cache_rx) = std::sync::mpsc::channel();
    let app = DawApp {
        workspace_kind: WorkspaceKind::Daily,
        daily_page_date: None,
        config_app_dir: Some(config_app_dir.to_path_buf()),
        editor: crate::editor::DawEditorState::new(data, 0, 0, TRACKS, MEASURES),
        mode: DawMode::Normal,
        help_origin: DawMode::Normal,
        sound_check_guide: cmrt_tui_core::sound_check_guide::SoundCheckGuide::new(None),
        textarea: TextArea::default(),
        cfg: Arc::new(cmrt_runtime::Config {
            sample_rate: 44_100.0,
            ..Default::default()
        }),
        plugin_entries: cmrt_offline_render::PluginEntries::none(),
        cache: Arc::new(Mutex::new(vec![
            vec![CellCache::empty(); MEASURES + 1];
            TRACKS
        ])),
        cache_tx,
        cache_render_workers: cmrt_runtime::DEFAULT_OFFLINE_RENDER_WORKERS,
        render_queue: crate::render_queue::RenderQueue::disabled_for_tests(),
        playback: crate::playback_runtime::DawPlaybackRuntime::for_test(TRACKS, MEASURES),
        log_lines: Arc::new(Mutex::new(VecDeque::new())),
        track_rerender_batches: Arc::new(Mutex::new(vec![None; TRACKS])),
        solo_tracks: vec![false; TRACKS],
        track_volumes_db: vec![0; TRACKS],
        pending_auto_trim: false,
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
    };
    (app, cache_rx)
}

fn build_daily_app(config_app_dir: &Path) -> DawApp {
    build_daily_app_with_cache_jobs(config_app_dir).0
}

fn prepare_saved_daily(config_app_dir: &Path, page_date: &str, mml: &str) {
    let mut app = build_daily_app(config_app_dir);
    app.daily_page_date = Some(page_date.to_owned());
    app.editor.data[1][1] = mml.to_owned();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 2;
    app.save_daily_recovery().unwrap();
}
