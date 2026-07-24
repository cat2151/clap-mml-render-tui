use super::*;

#[test]
fn handle_normal_r_shows_error_when_patches_dirs_is_missing() {
    let mut cfg = test_config();
    cfg.patches_dirs = None;
    let mut app = NotepadScreen::new_for_test(cfg);

    app.handle_normal(KeyCode::Char('r'));

    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Err(msg) if msg == "patches_dirs が設定されていません"
    ));
}

#[test]
fn handle_normal_r_shows_error_while_patches_are_loading() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Loading));

    app.handle_normal(KeyCode::Char('r'));

    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Err(msg) if msg == "パッチを読み込み中です..."
    ));
}

#[test]
fn handle_normal_r_shows_error_when_patch_loading_failed() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Err("boom".to_string())));

    app.handle_normal(KeyCode::Char('r'));

    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Err(msg) if msg == "パッチの読み込みに失敗: boom"
    ));
}

#[test]
fn handle_normal_r_shows_error_when_patch_list_is_empty() {
    let mut app = NotepadScreen::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(Vec::new())));

    app.handle_normal(KeyCode::Char('r'));

    assert!(matches!(
        &*app.playback.session.play_state().lock().unwrap(),
        PlayState::Err(msg) if msg == "patches_dirs にパッチが見つかりません"
    ));
}
