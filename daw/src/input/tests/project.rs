use super::*;

fn unique_temp_dir(name: &str) -> TempDirGuard {
    TempDirGuard::new(&format!(
        "cmrt_daw_project_{name}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

#[test]
fn f_opens_project_overlay_and_escape_returns_to_normal() {
    let (mut app, _cache_rx) = build_test_app();

    app.handle_normal(KeyCode::Char('f'));
    assert_eq!(app.mode, DawMode::Project);
    assert!(app.overlays.project.action.is_none());

    app.handle_project_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert_eq!(app.mode, DawMode::Normal);
}

#[test]
fn save_as_then_open_restores_grid_and_mixer_from_one_file() {
    let tmp = unique_temp_dir("roundtrip");
    std::fs::create_dir_all(tmp.path()).unwrap();
    let _history_guard = cmrt_history::test_support::set_local_dir_envs(tmp.path());
    let project_path = tmp.path().join("song.cmrt-daw.json");
    let (mut app, _cache_rx) = build_test_app();
    app.editor.data[0][0] = r#"{"beat":"4/4"}t132"#.to_string();
    app.editor.data[2][0] = r#"{"Surge XT patch":"Piano"}"#.to_string();
    app.editor.data[2][2] = "l8cdef".to_string();
    app.track_volumes_db[2] = -6;

    app.start_project_overlay();
    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    app.overlays.project.path_textarea =
        cmrt_tui_core::text_input::new_single_line_textarea(&project_path.to_string_lossy());
    app.handle_project_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.mode, DawMode::Normal);
    let json = std::fs::read_to_string(&project_path).unwrap();
    assert!(json.contains("clap-mml-render-tui.daw-project"));
    assert!(json.contains("l8cdef"));

    app.editor.data[0][0] = "t60".to_string();
    app.editor.data[2][2].clear();
    app.track_volumes_db[2] = 3;
    app.start_project_overlay();
    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
    app.handle_project_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.mode, DawMode::Normal);
    assert_eq!(app.editor.data[0][0], r#"{"beat":"4/4"}t132"#);
    assert_eq!(app.editor.data[2][2], "l8cdef");
    assert_eq!(app.track_volumes_db[2], -6);
}

#[test]
fn save_as_renames_existing_file_and_notice_lives_with_project_overlay() {
    let tmp = unique_temp_dir("backup");
    std::fs::create_dir_all(tmp.path()).unwrap();
    let project_path = tmp.path().join("song.cmrt-daw.json");
    let backup_path = tmp.path().join("song.cmrt-daw.json.bak");
    std::fs::write(&project_path, "old project").unwrap();
    let (mut app, _cache_rx) = build_test_app();
    app.editor.data[2][1] = "new project".to_string();

    app.start_project_overlay();
    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    app.overlays.project.path_textarea =
        cmrt_tui_core::text_input::new_single_line_textarea(&project_path.to_string_lossy());
    app.handle_project_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(
        std::fs::read_to_string(&backup_path).unwrap(),
        "old project"
    );
    assert!(std::fs::read_to_string(&project_path)
        .unwrap()
        .contains("new project"));
    assert_eq!(app.mode, DawMode::Project);
    assert!(app.overlays.project.action.is_none());
    assert_eq!(
        app.overlays.project.backup_notice_path.as_deref(),
        Some(backup_path.as_path())
    );

    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    assert_eq!(
        app.overlays.project.action,
        Some(DawProjectFileAction::SaveAs)
    );
    assert_eq!(
        app.overlays.project.backup_notice_path.as_deref(),
        Some(backup_path.as_path())
    );
    app.handle_project_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.handle_project_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert_eq!(app.mode, DawMode::Normal);
    assert!(app.overlays.project.backup_notice_path.is_none());
}

#[test]
fn save_as_uses_numbered_backup_when_bak_already_exists() {
    let tmp = unique_temp_dir("numbered_backup");
    std::fs::create_dir_all(tmp.path()).unwrap();
    let project_path = tmp.path().join("song.cmrt-daw.json");
    let first_backup_path = tmp.path().join("song.cmrt-daw.json.bak");
    let numbered_backup_path = tmp.path().join("song.cmrt-daw.json.bak.1");
    std::fs::write(&project_path, "latest old project").unwrap();
    std::fs::write(&first_backup_path, "older project").unwrap();
    let (app, _cache_rx) = build_test_app();

    let saved = app
        .save_project_as(&project_path.to_string_lossy())
        .unwrap();

    assert_eq!(
        saved.backup_path.as_deref(),
        Some(numbered_backup_path.as_path())
    );
    assert_eq!(
        std::fs::read_to_string(&first_backup_path).unwrap(),
        "older project"
    );
    assert_eq!(
        std::fs::read_to_string(&numbered_backup_path).unwrap(),
        "latest old project"
    );
}

#[test]
fn invalid_open_keeps_current_project_untouched_and_shows_error() {
    let tmp = unique_temp_dir("invalid");
    std::fs::create_dir_all(tmp.path()).unwrap();
    let project_path = tmp.path().join("broken.cmrt-daw.json");
    std::fs::write(&project_path, r#"{"format":"wrong"}"#).unwrap();
    let (mut app, _cache_rx) = build_test_app();
    app.editor.data[2][1] = "keep me".to_string();
    app.overlays.project.current_path = Some(project_path.clone());

    app.start_project_overlay();
    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
    app.handle_project_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.mode, DawMode::Project);
    assert_eq!(app.editor.data[2][1], "keep me");
    assert!(app.overlays.project.error.is_some());
}

#[test]
fn open_selector_filters_files_and_supports_vim_navigation() {
    let tmp = unique_temp_dir("selector");
    let child = tmp.path().join("child");
    std::fs::create_dir_all(&child).unwrap();
    let first = tmp.path().join("a.cmrt-daw.json");
    let second = tmp.path().join("b.cmrt-daw.json");
    std::fs::write(&first, "{}").unwrap();
    std::fs::write(&second, "{}").unwrap();
    std::fs::write(tmp.path().join("other.json"), "{}").unwrap();
    let (mut app, _cache_rx) = build_test_app();
    app.overlays.project.current_path = Some(first.clone());

    app.start_project_overlay();
    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));

    let explorer = app.overlays.project.file_explorer.as_ref().unwrap();
    assert_eq!(explorer.current().path, first);
    assert!(explorer.files().iter().any(|file| file.path == second));
    assert!(explorer.files().iter().any(|file| file.path == child));
    assert!(!explorer
        .files()
        .iter()
        .any(|file| file.name == "other.json"));

    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE));
    assert_eq!(
        app.overlays.project.selected_path().as_deref(),
        Some(first.as_path())
    );

    let child_index = app
        .overlays
        .project
        .file_explorer
        .as_ref()
        .unwrap()
        .files()
        .iter()
        .position(|file| file.path == child)
        .unwrap();
    app.overlays
        .project
        .file_explorer
        .as_mut()
        .unwrap()
        .set_selected_idx(child_index);
    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
    assert_eq!(
        app.overlays.project.file_explorer.as_ref().unwrap().cwd(),
        &child
    );

    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
    assert_eq!(
        app.overlays.project.file_explorer.as_ref().unwrap().cwd(),
        tmp.path()
    );
}

#[test]
fn slash_filter_commits_with_enter_and_escape_restores_previous_query() {
    let tmp = unique_temp_dir("filter");
    std::fs::create_dir_all(tmp.path()).unwrap();
    let alpha = tmp.path().join("alpha-song.cmrt-daw.json");
    let beta = tmp.path().join("beta-song.cmrt-daw.json");
    std::fs::write(&alpha, "{}").unwrap();
    std::fs::write(&beta, "{}").unwrap();
    let (mut app, _cache_rx) = build_test_app();
    app.overlays.project.current_path = Some(alpha.clone());

    app.start_project_overlay();
    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
    assert!(app.overlays.project.filter_active);
    assert!(app.uses_textarea_cursor());

    for character in "alpha".chars() {
        app.handle_project_key_event(KeyEvent::new(KeyCode::Char(character), KeyModifiers::NONE));
    }
    let explorer = app.overlays.project.file_explorer.as_ref().unwrap();
    assert!(explorer.files().iter().any(|file| file.path == alpha));
    assert!(!explorer.files().iter().any(|file| file.path == beta));

    app.handle_project_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert!(!app.overlays.project.filter_active);
    assert_eq!(app.overlays.project.query, "alpha");
    assert!(!app.uses_textarea_cursor());

    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE));
    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
    assert_eq!(app.overlays.project.query, "alphaz");
    app.handle_project_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert!(!app.overlays.project.filter_active);
    assert_eq!(app.overlays.project.query, "alpha");
    assert!(app
        .overlays
        .project
        .file_explorer
        .as_ref()
        .unwrap()
        .files()
        .iter()
        .any(|file| file.path == alpha));
}

#[test]
fn open_selector_automatically_previews_without_applying_project() {
    let tmp = unique_temp_dir("auto_preview");
    std::fs::create_dir_all(tmp.path()).unwrap();
    let project_path = tmp.path().join("preview.cmrt-daw.json");
    let (mut app, _cache_rx) = build_test_app();
    app.editor.data[0][0] = "t150".to_string();
    app.editor.data[2][2] = "cdef".to_string();
    app.save_project_as(&project_path.to_string_lossy())
        .unwrap();
    app.editor.data[2][2] = "still-current".to_string();
    app.overlays.project.current_path = Some(project_path);

    app.start_project_overlay();
    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));

    assert!(*app.playback.play_state.lock().unwrap() == DawPlayState::Preview);
    assert_eq!(
        app.playback
            .position
            .lock()
            .unwrap()
            .as_ref()
            .unwrap()
            .measure_index,
        1
    );
    assert!(app
        .overlays
        .project
        .preview_info
        .as_deref()
        .unwrap()
        .contains("preview: meas2"));
    assert_eq!(app.editor.data[2][2], "still-current");

    app.handle_project_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    assert!(*app.playback.play_state.lock().unwrap() == DawPlayState::Idle);
}

#[test]
fn manual_project_preview_starts_only_with_space() {
    let tmp = unique_temp_dir("manual_preview");
    std::fs::create_dir_all(tmp.path()).unwrap();
    let project_path = tmp.path().join("preview.cmrt-daw.json");
    let (mut app, _cache_rx) = build_test_app();
    app.editor.data[2][1] = "g".to_string();
    app.save_project_as(&project_path.to_string_lossy())
        .unwrap();
    app.overlays.project.current_path = Some(project_path);
    app.overlays.project.auto_preview = false;

    app.start_project_overlay();
    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
    assert!(*app.playback.play_state.lock().unwrap() == DawPlayState::Idle);
    assert!(app.overlays.project.preview_info.is_none());

    app.handle_project_key_event(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));

    assert!(*app.playback.play_state.lock().unwrap() == DawPlayState::Preview);
    assert!(app.overlays.project.preview_info.is_some());

    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    assert!(app.overlays.project.auto_preview);
    assert!(*app.playback.play_state.lock().unwrap() == DawPlayState::Preview);

    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    assert!(!app.overlays.project.auto_preview);
    assert!(*app.playback.play_state.lock().unwrap() == DawPlayState::Idle);
}

#[test]
fn open_resizes_all_project_dependent_buffers() {
    let tmp = unique_temp_dir("resize");
    std::fs::create_dir_all(tmp.path()).unwrap();
    let _history_guard = cmrt_history::test_support::set_local_dir_envs(tmp.path());
    let project_path = tmp.path().join("large.cmrt-daw.json");
    std::fs::write(
        &project_path,
        r#"{
          "format": "clap-mml-render-tui.daw-project",
          "format_version": 1,
          "project": {
            "track_count": 4,
            "playable_measure_count": 3,
            "tracks": [
              {"track_index":0,"role":"global_header","volume_db":0,"non_empty_cells":[{"measure_index":0,"role":"initialization","mml":"t140"}]},
              {"track_index":1,"role":"instrument","volume_db":-3,"non_empty_cells":[]},
              {"track_index":2,"role":"instrument","volume_db":0,"non_empty_cells":[]},
              {"track_index":3,"role":"instrument","volume_db":6,"non_empty_cells":[{"measure_index":3,"role":"playable_measure","mml":"g"}]}
            ]
          }
        }"#,
    )
    .unwrap();
    let (mut app, _cache_rx) = build_test_app();

    app.open_project(&project_path.to_string_lossy()).unwrap();

    // 保存ファイルの track_count は 4。グリッドは chord 行のぶん 1 行増えて 5 になる。
    assert_eq!((app.editor.tracks, app.editor.measures), (5, 3));
    assert_eq!(app.editor.data.len(), 5);
    assert!(app.editor.data.iter().all(|row| row.len() == 4));
    assert_eq!(app.editor.data[4][3], "g");
    assert_eq!(app.track_volumes_db, vec![0, 0, -3, 0, 6]);
    assert_eq!(app.solo_tracks, vec![false; 5]);
    let cache = app.cache.lock().unwrap();
    assert_eq!(cache.len(), 5);
    assert!(cache.iter().all(|row| row.len() == 4));
    drop(cache);
    assert_eq!(app.playback.measure_mmls.lock().unwrap().len(), 3);
    assert!(app
        .playback
        .measure_track_mmls
        .lock()
        .unwrap()
        .iter()
        .all(|tracks| tracks.len() == 5));
    assert_eq!(app.playback_track_gains().len(), 5);
}

// ─── chord 行 ────────────────────────────────────────────────

/// chord 行に書いた内容は project file に保存され、開き直すと chord 行へ戻る。
#[test]
fn a_chord_row_survives_saving_and_opening_a_project_file() {
    let tmp = unique_temp_dir("chord_row");
    std::fs::create_dir_all(tmp.path()).unwrap();
    let _history_guard = cmrt_history::test_support::set_local_dir_envs(tmp.path());
    let project_path = tmp.path().join("chord.cmrt-daw.json");
    let (mut app, _cache_rx) = build_test_app();
    app.editor.data[0][0] = r#"{"beat":"4/4"}t120"#.to_string();
    app.editor.data[crate::CHORD_TRACK][0] = "key:G".to_string();
    app.editor.data[crate::CHORD_TRACK][1] = "I-IV-V-I".to_string();
    app.editor.data[crate::FIRST_PLAYABLE_TRACK][1] = "cde".to_string();

    app.save_project_as(&project_path.to_string_lossy())
        .unwrap();
    app.editor.data[crate::CHORD_TRACK][0].clear();
    app.editor.data[crate::CHORD_TRACK][1].clear();
    app.open_project(&project_path.to_string_lossy()).unwrap();

    assert_eq!(app.editor.data[crate::CHORD_TRACK][0], "key:G");
    assert_eq!(app.editor.data[crate::CHORD_TRACK][1], "I-IV-V-I");
    assert_eq!(app.editor.data[crate::FIRST_PLAYABLE_TRACK][1], "cde");
}

/// chord 行を使わなければ project file に chord_track は現れない。
#[test]
fn an_empty_chord_row_is_left_out_of_the_project_file() {
    let tmp = unique_temp_dir("chord_row_absent");
    std::fs::create_dir_all(tmp.path()).unwrap();
    let _history_guard = cmrt_history::test_support::set_local_dir_envs(tmp.path());
    let project_path = tmp.path().join("plain.cmrt-daw.json");
    let (mut app, _cache_rx) = build_test_app();
    app.editor.data[0][0] = r#"{"beat":"4/4"}t120"#.to_string();
    app.editor.data[crate::FIRST_PLAYABLE_TRACK][1] = "cde".to_string();

    app.save_project_as(&project_path.to_string_lossy())
        .unwrap();

    let json = std::fs::read_to_string(&project_path).unwrap();
    assert!(!json.contains("chord_track"), "json: {json}");
    // 演奏 track の保存番号は chord 行のぶんずれない（画面の T1 = "track_index": 1）
    assert!(json.contains(r#""track_index": 1"#), "json: {json}");
}
