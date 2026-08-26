use std::{
    io::{Error, Write},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde_json::{json, Value};

use super::*;
use crate::{CacheState, CellCache, DawMode, WorkspaceKind, DEFAULT_TRACK0_MML, MEASURES, TRACKS};
use ratatui_textarea::TextArea;
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

struct TempDirectory(PathBuf);

impl TempDirectory {
    fn new(label: &str) -> Self {
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

    fn path(&self) -> &Path {
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

fn build_daily_app(config_app_dir: &Path) -> DawApp {
    let mut data = vec![vec![String::new(); MEASURES + 1]; TRACKS];
    data[0][0] = DEFAULT_TRACK0_MML.to_string();
    let (cache_tx, _cache_rx) = std::sync::mpsc::channel();
    DawApp {
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
        overlays: crate::overlays::DawOverlays::new(1),
        patch_phrase_store: cmrt_history::PatchPhraseStore::default(),
        patch_phrase_store_dirty: false,
        random_patch_decks: cmrt_tui_core::random::RandomIndexDecks::default(),
    }
}

fn prepare_saved_daily(config_app_dir: &Path, page_date: &str, mml: &str) {
    let mut app = build_daily_app(config_app_dir);
    app.daily_page_date = Some(page_date.to_owned());
    app.editor.data[1][1] = mml.to_owned();
    app.editor.cursor_track = 1;
    app.editor.cursor_measure = 2;
    app.save_daily_recovery().unwrap();
}

#[test]
fn daily_paths_share_one_feature_root_and_flat_archive() {
    let config_dir = Path::new("config-root");

    assert_eq!(daily_feature_root(config_dir), config_dir.join("daily_daw"));
    assert_eq!(
        daily_current_path(config_dir),
        config_dir.join("daily_daw").join("current.json")
    );
    assert_eq!(
        daily_archive_root(config_dir),
        config_dir.join("daily_daw").join("archive")
    );
    assert_eq!(
        daily_archive_path(config_dir, "2026-08-26").unwrap(),
        config_dir
            .join("daily_daw")
            .join("archive")
            .join("2026-08-26.cmrt-daw.json")
    );
}

#[test]
fn recovery_wire_roundtrips_existing_project_and_cache_shapes() {
    let recovery = recovery("2026-08-26", "cdef");

    let encoded = serde_json::to_value(&recovery).unwrap();
    let decoded = decode_daily_recovery(&serde_json::to_string(&encoded).unwrap()).unwrap();

    assert_eq!(encoded, recovery_value("2026-08-26", "cdef"));
    assert_eq!(decoded.page_date, "2026-08-26");
    assert_eq!((decoded.cursor_track, decoded.cursor_measure), (1, 2));
    assert_eq!(decoded.cached_measures.len(), 1);
    assert_eq!(decoded.cached_measures[0].mml_hash, 42);
}

#[test]
fn recovery_rejects_invalid_date_and_invalid_nested_project() {
    for date in ["2026-8-26", "2026-02-29", "2026-13-01", "../escape"] {
        let error =
            decode_daily_recovery(&serde_json::to_string(&recovery_value(date, "cdef")).unwrap())
                .unwrap_err()
                .to_string();
        assert!(error.contains("YYYY-MM-DD"), "{date}: {error}");
    }

    let mut invalid_project = recovery_value("2026-08-26", "cdef");
    invalid_project["project_file"]["format"] = json!("not-a-daw-project");
    let error = decode_daily_recovery(&serde_json::to_string(&invalid_project).unwrap())
        .unwrap_err()
        .to_string();
    assert!(error.contains("project が不正です"));
}

#[test]
fn missing_recovery_is_first_use_and_invalid_files_are_errors() {
    let temp = TempDirectory::new("load");
    let current_path = daily_current_path(temp.path());

    assert!(load_daily_recovery(&current_path).unwrap().is_none());

    std::fs::create_dir_all(current_path.parent().unwrap()).unwrap();
    std::fs::write(&current_path, b"not json").unwrap();
    let error = load_daily_recovery(&current_path).unwrap_err().to_string();
    assert!(error.contains("Daily recovery が不正です"));
}

#[test]
fn date_classification_handles_first_use_resume_and_rollover() {
    assert_eq!(
        classify_daily_date(None, "2026-08-26").unwrap(),
        DailyDateClassification::FirstUse
    );
    assert_eq!(
        classify_daily_date(Some("2026-08-26"), "2026-08-26").unwrap(),
        DailyDateClassification::Resume
    );
    assert_eq!(
        classify_daily_date(Some("2026-08-27"), "2026-08-26").unwrap(),
        DailyDateClassification::Resume
    );
    assert_eq!(
        classify_daily_date(Some("2026-08-25"), "2026-08-26").unwrap(),
        DailyDateClassification::Rollover
    );
    assert!(classify_daily_date(Some("2026-02-29"), "2026-08-26").is_err());
    assert!(classify_daily_date(None, "today").is_err());
}

#[test]
fn archive_create_new_never_overwrites_an_existing_snapshot() {
    let temp = TempDirectory::new("already-exists");
    let path = daily_archive_path(temp.path(), "2026-08-26").unwrap();
    let first = recovery("2026-08-26", "cdef");
    let second = recovery("2026-08-26", "gggg");

    assert_eq!(
        write_daily_archive(&path, &first.project_file).unwrap(),
        DailyArchiveOutcome::Created
    );
    let archived = std::fs::read(&path).unwrap();
    assert_eq!(
        write_daily_archive(&path, &second.project_file).unwrap(),
        DailyArchiveOutcome::AlreadyExists
    );

    assert_eq!(std::fs::read(&path).unwrap(), archived);
    let value: Value = serde_json::from_slice(&archived).unwrap();
    assert_eq!(value, project_value("cdef"));
}

#[test]
fn archive_write_failure_removes_partial_file_and_keeps_recovery() {
    let temp = TempDirectory::new("write-failure");
    let recovery = recovery("2026-08-26", "cdef");
    let recovery_before = serde_json::to_vec(&recovery).unwrap();
    let current_path = daily_current_path(temp.path());
    std::fs::create_dir_all(current_path.parent().unwrap()).unwrap();
    std::fs::write(&current_path, &recovery_before).unwrap();
    let path = daily_archive_path(temp.path(), "2026-08-26").unwrap();
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();

    let error = create_new_archive_with(&path, |file| {
        file.write_all(b"{\"partial\":")?;
        Err(Error::other("injected write failure"))
    })
    .unwrap_err()
    .to_string();

    assert!(error.contains("injected write failure"));
    assert!(!path.exists());
    assert_eq!(serde_json::to_vec(&recovery).unwrap(), recovery_before);
    assert_eq!(std::fs::read(current_path).unwrap(), recovery_before);
}

#[test]
fn first_entry_writes_the_standard_blank_daily_recovery() {
    let temp = TempDirectory::new("first-entry");
    let mut app = build_daily_app(temp.path());

    app.initialize_daily_workspace("2026-08-26");

    assert_eq!(app.workspace_kind(), WorkspaceKind::Daily);
    assert_eq!(app.daily_page_date(), Some("2026-08-26"));
    assert_eq!(app.editor.data[0][0], DEFAULT_TRACK0_MML);
    assert!(app.editor.data[1][1].is_empty());
    let saved = load_daily_recovery(&daily_current_path(temp.path()))
        .unwrap()
        .unwrap();
    assert_eq!(saved.page_date, "2026-08-26");
    assert!(app
        .log_lines
        .lock()
        .unwrap()
        .iter()
        .all(|line| !line.contains("daily rollover")));
}

#[test]
fn same_day_entry_restores_project_cursor_and_daily_cache_without_persistent_writes() {
    let temp = TempDirectory::new("same-day-cache");
    let _env_guard = cmrt_history::test_support::set_local_dir_envs(temp.path());
    let config_app_dir = temp.path().join("clap-mml-render-tui");
    let mut first = build_daily_app(&config_app_dir);
    first.daily_page_date = Some("2026-08-26".to_owned());
    first.editor.data[1][1] = "cdef".to_owned();
    first.editor.cursor_track = 1;
    first.editor.cursor_measure = 1;
    let mml_hash = cmrt_history::daw_cache_mml_hash(&first.build_cell_mml(1, 1));
    {
        let mut cache = first.cache.lock().unwrap();
        cache[1][1].state = CacheState::Ready;
        cache[1][1].rendered_mml_hash = Some(mml_hash);
    }
    let persistent_cache_dir =
        crate::cache::ensure_workspace_cache_dir(WorkspaceKind::Persistent).unwrap();
    let daily_cache_dir = crate::cache::ensure_workspace_cache_dir(WorkspaceKind::Daily).unwrap();
    let daily_wav = daily_cache_dir.join("track1_meas1.wav");
    cmrt_core::write_wav(&[0.25, -0.25], 44_100, &daily_wav).unwrap();
    first.save();
    first.editor.cursor_measure = 2;
    first.save_history_state();
    drop(first);

    let mut resumed = build_daily_app(&config_app_dir);
    resumed.initialize_daily_workspace("2026-08-26");

    assert_eq!(resumed.editor.data[1][1], "cdef");
    assert_eq!(
        (resumed.editor.cursor_track, resumed.editor.cursor_measure),
        (1, 2)
    );
    assert!(resumed.cache.lock().unwrap()[1][1].state == CacheState::Ready);
    assert!(resumed.cache.lock().unwrap()[1][1].samples.is_some());
    assert!(daily_wav.exists());
    assert!(!persistent_cache_dir.join("track1_meas1.wav").exists());
    assert!(daily_current_path(&config_app_dir).exists());
    assert!(!cmrt_history::daw_file_path().unwrap().exists());
    assert!(!config_app_dir
        .join("history")
        .join("history_daw.json")
        .exists());
}

#[test]
fn later_entry_archives_before_replacing_the_page_with_blank() {
    let temp = TempDirectory::new("rollover");
    prepare_saved_daily(temp.path(), "2026-08-25", "cdef");
    let mut app = build_daily_app(temp.path());

    app.initialize_daily_workspace("2026-08-26");

    assert_eq!(app.daily_page_date(), Some("2026-08-26"));
    assert_eq!(app.editor.data[0][0], DEFAULT_TRACK0_MML);
    assert!(app.editor.data[1][1].is_empty());
    let archived =
        std::fs::read_to_string(daily_archive_path(temp.path(), "2026-08-25").unwrap()).unwrap();
    assert!(archived.contains("cdef"));
    let current = load_daily_recovery(&daily_current_path(temp.path()))
        .unwrap()
        .unwrap();
    assert_eq!(current.page_date, "2026-08-26");
    assert!(app
        .log_lines
        .lock()
        .unwrap()
        .iter()
        .any(|line| { line.starts_with("daily rollover: 2026-08-25 -> 2026-08-26; archive=") }));
}

#[test]
fn rollover_failure_keeps_old_page_and_invalid_recovery_starts_fresh() {
    let failure = TempDirectory::new("rollover-failure");
    prepare_saved_daily(failure.path(), "2026-08-25", "old-page");
    std::fs::write(
        daily_archive_root(failure.path()),
        b"blocks directory creation",
    )
    .unwrap();
    let mut kept = build_daily_app(failure.path());

    kept.initialize_daily_workspace("2026-08-26");

    assert_eq!(kept.daily_page_date(), Some("2026-08-25"));
    assert_eq!(kept.editor.data[1][1], "old-page");
    assert!(kept.log_lines.lock().unwrap().iter().any(|line| {
        line.starts_with("daily rollover failed:") && line.contains("keeping 2026-08-25")
    }));

    let invalid = TempDirectory::new("invalid-recovery");
    let current_path = daily_current_path(invalid.path());
    std::fs::create_dir_all(current_path.parent().unwrap()).unwrap();
    std::fs::write(&current_path, b"not json").unwrap();
    let mut fresh = build_daily_app(invalid.path());

    fresh.initialize_daily_workspace("2026-08-26");

    assert_eq!(fresh.daily_page_date(), Some("2026-08-26"));
    assert_eq!(fresh.editor.data[0][0], DEFAULT_TRACK0_MML);
    assert!(fresh.editor.data[1][1].is_empty());
    assert!(fresh.log_lines.lock().unwrap().iter().any(|line| {
        line.starts_with("daily recovery failed:")
            && line.contains(&current_path.display().to_string())
    }));
    assert!(load_daily_recovery(&current_path).unwrap().is_some());
}
