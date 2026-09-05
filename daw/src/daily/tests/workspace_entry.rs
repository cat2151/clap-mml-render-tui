//! `initialize_daily_workspace()` の入口 3 種（初回 / 同日 Resume / 翌日 Rollover）と失敗経路。

use super::*;
use crate::CacheState;

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
    // rollover が成功すると `daw_cache/<plugin>/daily/` を掃除するので、
    // **env lock を取ってキャッシュの置き場を temp へ隔離しないと、
    // 同時に走っている別テストの WAV を消してしまう**（`CMRT_BASE_DIR` はプロセス共有）。
    let _env_guard = cmrt_history::test_support::set_local_dir_envs(temp.path());
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
