use super::*;

#[test]
fn handle_normal_g_inserts_generated_line_above_current_line_and_plays_it() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.cursor = 1;
    app.list_state.select(Some(1));
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "Pads/Pad 1.fxp",
    ]))));

    let result = app.handle_normal(KeyCode::Char('g'));

    assert!(matches!(result, NormalAction::Continue));
    assert_eq!(app.cursor, 1);
    assert_eq!(app.list_state.selected(), Some(1));
    assert_eq!(app.lines[0], "line 0");
    assert_eq!(app.lines[2], "line 1");
    let inserted = &app.lines[1];
    assert!(
        inserted == r#"{"Surge XT patch": "Pads/Pad 1.fxp"} c1"#
            || inserted == r#"{"Surge XT patch": "Pads/Pad 1.fxp"} cfg1"#,
        "inserted: {inserted}"
    );
    assert_eq!(
        app.patch_phrase_store.notepad.history,
        vec![inserted.clone()]
    );
    assert_eq!(
        app.patch_phrase_store
            .patches
            .get("Pads/Pad 1.fxp")
            .map(|state| state.history.clone()),
        Some(vec![inserted
            .strip_prefix(r#"{"Surge XT patch": "Pads/Pad 1.fxp"} "#)
            .unwrap()
            .to_string()])
    );
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == inserted
    ));
}

#[test]
fn handle_normal_g_shows_error_when_patches_are_unavailable() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(Vec::new())));

    let result = app.handle_normal(KeyCode::Char('g'));

    assert!(matches!(result, NormalAction::Continue));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Err(msg) if msg == "patches_dir にパッチが見つかりません"
    ));
}

#[test]
fn handle_normal_r_inserts_random_patch_at_start_of_plain_line() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["cde".to_string()];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "Pads/Pad 1.fxp",
    ]))));

    let result = app.handle_normal(KeyCode::Char('r'));

    assert!(matches!(result, NormalAction::Continue));
    assert_eq!(
        app.lines,
        vec![r#"{"Surge XT patch": "Pads/Pad 1.fxp"} cde"#.to_string()]
    );
}

#[test]
fn handle_normal_r_replaces_existing_patch_at_start_of_current_line() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Old/Pad.fxp"} cde"#.to_string()];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "Leads/Lead 1.fxp",
    ]))));

    let result = app.handle_normal(KeyCode::Char('r'));

    assert!(matches!(result, NormalAction::Continue));
    assert_eq!(
        app.lines,
        vec![r#"{"Surge XT patch": "Leads/Lead 1.fxp"} cde"#.to_string()]
    );
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Leads/Lead 1.fxp"} cde"#
    ));
}

#[test]
fn handle_normal_r_reapplies_same_patch_to_each_semicolon_branch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Old/Pad.fxp"} c;f"#.to_string()];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "Leads/Lead 1.fxp",
    ]))));

    let result = app.handle_normal(KeyCode::Char('r'));

    assert!(matches!(result, NormalAction::Continue));
    assert_eq!(
        app.lines,
        vec![
            r#"{"Surge XT patch": "Leads/Lead 1.fxp"} c;{"Surge XT patch": "Leads/Lead 1.fxp"} f"#
                .to_string()
        ]
    );
}

#[test]
fn handle_normal_r_replaces_spaced_semicolon_branch_patch_without_duplication() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![
        r#"{"Surge XT patch":"Old/Pad.fxp"} c; {"Surge XT patch":"Older/Lead.fxp"} f"#.to_string(),
    ];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "Leads/Lead 1.fxp",
    ]))));

    let result = app.handle_normal(KeyCode::Char('r'));

    assert!(matches!(result, NormalAction::Continue));
    assert_eq!(
        app.lines,
        vec![
            r#"{"Surge XT patch": "Leads/Lead 1.fxp"} c;{"Surge XT patch": "Leads/Lead 1.fxp"} f"#
                .to_string()
        ]
    );
}

#[test]
fn handle_normal_r_inserts_c_for_empty_line_before_playing() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![String::new()];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "Pads/Pad 1.fxp",
    ]))));

    let result = app.handle_normal(KeyCode::Char('r'));

    assert!(matches!(result, NormalAction::Continue));
    assert_eq!(
        app.lines,
        vec![r#"{"Surge XT patch": "Pads/Pad 1.fxp"} c"#.to_string()]
    );
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Pads/Pad 1.fxp"} c"#
    ));
}

#[test]
fn handle_normal_r_inserts_c_when_all_semicolon_branches_are_empty() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![" ; ".to_string()];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(make_patches(&[
        "Pads/Pad 1.fxp",
    ]))));

    let result = app.handle_normal(KeyCode::Char('r'));

    assert!(matches!(result, NormalAction::Continue));
    assert_eq!(
        app.lines,
        vec![r#"{"Surge XT patch": "Pads/Pad 1.fxp"} c"#.to_string()]
    );
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == r#"{"Surge XT patch": "Pads/Pad 1.fxp"} c"#
    ));
}

#[test]
fn handle_normal_r_shows_error_when_patches_dir_is_missing() {
    let mut cfg = test_config();
    cfg.patches_dir = None;
    let mut app = TuiApp::new_for_test(cfg);

    app.handle_normal(KeyCode::Char('r'));

    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Err(msg) if msg == "patches_dir が設定されていません"
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
        PlayState::Err(msg) if msg == "patches_dir にパッチが見つかりません"
    ));
}

#[test]
fn handle_normal_t_enters_patch_select_when_random_timbre_disabled() {
    let mut app = TuiApp::new_for_test(test_config());
    let patches = make_patches(&["Pads/Pad 1.fxp"]);
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(patches.clone())));

    app.handle_normal(KeyCode::Char('t'));

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(app.patch_all, patches);
    assert_eq!(app.patch_filtered, vec!["Pads/Pad 1.fxp"]);
}

#[test]
fn handle_normal_t_selects_current_line_patch_when_present() {
    let mut app = TuiApp::new_for_test(test_config());
    let patches = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    app.lines = vec![r#"{"Surge XT patch":"Leads/Lead 1.fxp"} l8cdef"#.to_string()];
    app.patch_load_state = Arc::new(Mutex::new(PatchLoadState::Ready(patches)));

    app.handle_normal(KeyCode::Char('t'));

    assert!(matches!(app.mode, Mode::PatchSelect));
    assert_eq!(app.patch_cursor, 1);
    assert_eq!(app.patch_list_state.selected(), Some(1));
}
