use super::*;

fn unique_temp_dir(name: &str) -> TempDirGuard {
    TempDirGuard::new(&format!(
        "cmrt_daw_daily_project_{name}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn save_archive_project(app: &mut DawApp, archive_path: &std::path::Path, mml: &str) {
    std::fs::create_dir_all(archive_path.parent().unwrap()).unwrap();
    app.editor.data[1][1] = mml.to_string();
    app.save_project_as(&archive_path.to_string_lossy())
        .unwrap();
}

#[test]
fn daily_f_and_project_overlay_entry_are_suppressed() {
    let (mut app, _cache_rx) = build_test_app();
    app.workspace_kind = crate::WorkspaceKind::Daily;
    app.daily_page_date = Some("2026-08-26".to_string());

    app.handle_normal(KeyCode::Char('f'));
    assert_eq!(app.mode, DawMode::Normal);
    assert!(app.overlays.project.action.is_none());

    app.start_project_overlay();
    assert_eq!(app.mode, DawMode::Normal);
    assert!(app.overlays.project.action.is_none());
}

#[test]
fn daily_archive_selector_starts_at_managed_root_without_changing_normal_open() {
    let tmp = unique_temp_dir("selector_roots");
    let archive_root = crate::daily::daily_archive_root(tmp.path());
    let archive_path = archive_root.join("2026-08-25.cmrt-daw.json");
    let normal_root = tmp.path().join("normal");
    let normal_path = normal_root.join("song.cmrt-daw.json");
    std::fs::create_dir_all(&normal_root).unwrap();
    let (mut app, _cache_rx) = build_test_app();
    app.config_app_dir = Some(tmp.path().to_path_buf());
    save_archive_project(&mut app, &archive_path, "archive");
    app.save_project_as(&normal_path.to_string_lossy()).unwrap();
    app.overlays.project.current_path = Some(normal_path.clone());

    app.start_project_overlay();
    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    assert_eq!(
        app.overlays.project.action,
        Some(DawProjectFileAction::OpenDailyArchive)
    );
    assert_eq!(
        app.overlays.project.file_explorer.as_ref().unwrap().cwd(),
        &archive_root
    );

    app.handle_project_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
    assert_eq!(
        app.overlays.project.action,
        Some(DawProjectFileAction::Open)
    );
    assert_eq!(
        app.overlays.project.file_explorer.as_ref().unwrap().cwd(),
        &normal_root
    );
    assert_eq!(
        app.overlays.project.selected_path().as_deref(),
        Some(normal_path.as_path())
    );

    app.handle_project_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
    assert_eq!(
        app.overlays.project.current_path.as_deref(),
        Some(normal_path.as_path())
    );
    assert!(app
        .log_lines
        .lock()
        .unwrap()
        .iter()
        .any(|line| line == &format!("project opened: {}", normal_path.display())));
}

#[test]
fn daily_archive_open_applies_copy_autosaves_and_clears_project_path() {
    let tmp = unique_temp_dir("copy_open");
    std::fs::create_dir_all(tmp.path()).unwrap();
    let _history_guard = cmrt_history::test_support::set_local_dir_envs(tmp.path());
    let archive_path =
        crate::daily::daily_archive_root(tmp.path()).join("2026-08-25.cmrt-daw.json");
    let (mut app, _cache_rx) = build_test_app();
    app.config_app_dir = Some(tmp.path().to_path_buf());
    save_archive_project(&mut app, &archive_path, "archive melody");
    app.editor.data[1][1] = "persistent melody".to_string();
    app.overlays.project.current_path = Some(tmp.path().join("named.cmrt-daw.json"));

    app.start_project_overlay();
    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    assert!(*app.playback.play_state.lock().unwrap() == DawPlayState::Preview);
    app.handle_project_key_event(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));

    assert_eq!(app.mode, DawMode::Normal);
    assert_eq!(app.editor.data[1][1], "archive melody");
    assert!(app.overlays.project.current_path.is_none());
    assert!(*app.playback.play_state.lock().unwrap() == DawPlayState::Idle);
    assert!(app.log_lines.lock().unwrap().iter().any(|line| {
        line == &format!("daily archive opened as copy: {}", archive_path.display())
    }));
    let autosave = std::fs::read_to_string(cmrt_history::daw_file_path().unwrap()).unwrap();
    assert!(autosave.contains("archive melody"));
    assert!(!autosave.contains("persistent melody"));

    app.start_project_overlay();
    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
    assert_eq!(
        cmrt_tui_core::text_input::textarea_value(&app.overlays.project.path_textarea),
        crate::project::DEFAULT_PROJECT_FILE_NAME
    );
}

#[test]
fn cancelling_daily_archive_selector_uses_existing_preview_cancellation() {
    let tmp = unique_temp_dir("preview_cancel");
    let archive_path =
        crate::daily::daily_archive_root(tmp.path()).join("2026-08-25.cmrt-daw.json");
    let (mut app, _cache_rx) = build_test_app();
    app.config_app_dir = Some(tmp.path().to_path_buf());
    save_archive_project(&mut app, &archive_path, "cdef");

    app.start_project_overlay();
    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
    app.handle_project_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    assert!(*app.playback.play_state.lock().unwrap() == DawPlayState::Preview);

    app.handle_project_key_event(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));

    assert!(*app.playback.play_state.lock().unwrap() == DawPlayState::Idle);
    assert!(app.overlays.project.action.is_none());
    assert!(app.overlays.project.file_explorer.is_none());
}
