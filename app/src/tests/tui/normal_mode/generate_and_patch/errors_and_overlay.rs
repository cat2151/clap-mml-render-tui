use super::*;

#[test]
fn handle_normal_r_shows_error_when_patches_dirs_is_missing() {
    let mut cfg = test_config();
    cfg.patches_dirs = None;
    let mut app = TuiApp::new_for_test(cfg);

    app.handle_normal(KeyCode::Char('r'));

    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Err(msg) if msg == "patches_dirs が設定されていません"
    ));
}

#[test]
fn handle_normal_r_shows_error_while_patches_are_loading() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Loading));

    app.handle_normal(KeyCode::Char('r'));

    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Err(msg) if msg == "パッチを読み込み中です..."
    ));
}

#[test]
fn handle_normal_r_shows_error_when_patch_loading_failed() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Err("boom".to_string())));

    app.handle_normal(KeyCode::Char('r'));

    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Err(msg) if msg == "パッチの読み込みに失敗: boom"
    ));
}

#[test]
fn handle_normal_r_shows_error_when_patch_list_is_empty() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(Vec::new())));

    app.handle_normal(KeyCode::Char('r'));

    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Err(msg) if msg == "patches_dirs にパッチが見つかりません"
    ));
}

#[test]
fn handle_normal_t_enters_patch_select_when_random_timbre_disabled() {
    let mut app = TuiApp::new_for_test(test_config());
    let patches = make_patches(&["Pads/Pad 1.fxp"]);
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(patches.clone())));

    app.handle_normal(KeyCode::Char('t'));

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(app.patch_select.patch_all, patches);
    assert_eq!(app.patch_select.patch_filtered, vec!["Pads/Pad 1.fxp"]);
}

#[test]
fn handle_normal_t_selects_current_line_patch_when_present() {
    let mut app = TuiApp::new_for_test(test_config());
    let patches = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    app.lines = vec![r#"{"Surge XT patch":"Leads/Lead 1.fxp"} l8cdef"#.to_string()];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(patches)));

    app.handle_normal(KeyCode::Char('t'));

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(app.patch_select.patch_cursor, 1);
    assert_eq!(app.patch_select.patch_list_state.selected(), Some(1));
}
