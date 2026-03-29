use super::*;

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

#[test]
fn handle_normal_question_mark_enters_help_mode() {
    let mut app = TuiApp::new_for_test(test_config());

    let result = app.handle_normal(KeyCode::Char('?'));

    assert!(matches!(result, NormalAction::Continue));
    assert!(matches!(app.mode, Mode::Help));
}

#[test]
fn handle_normal_page_down_and_page_up_move_by_visible_page() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = (0..8).map(|i| format!("line {i}")).collect();
    app.normal_page_size = 3;
    app.cursor = 1;
    app.list_state.select(Some(1));

    app.handle_normal(KeyCode::PageDown);
    assert_eq!(app.cursor, 4);
    assert_eq!(app.list_state.selected(), Some(4));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "line 4"
    ));

    app.handle_normal(KeyCode::PageUp);
    assert_eq!(app.cursor, 1);
    assert_eq!(app.list_state.selected(), Some(1));
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "line 1"
    ));
}

#[test]
fn handle_normal_f_shows_error_when_current_line_has_no_patch_json() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["cde".to_string()];

    app.handle_normal(KeyCode::Char('f'));

    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Err(msg) if msg == "patch name JSON が見つかりません"
    ));
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn handle_normal_p_shows_error_when_yank_buffer_is_empty() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["cde".to_string()];

    app.handle_normal(KeyCode::Char('p'));

    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Err(msg) if msg == "yank バッファが空です"
    ));
    assert_eq!(app.lines, vec!["cde".to_string()]);
}

#[test]
fn handle_normal_f_enters_patch_phrase_for_current_patch() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} cde"#.to_string()];

    app.handle_normal(KeyCode::Char('f'));

    assert!(matches!(app.mode, Mode::PatchPhrase));
    assert_eq!(app.patch_phrase_name.as_deref(), Some("Pads/Pad 1.fxp"));
    assert_eq!(app.patch_phrase_history_items(), vec!["c".to_string()]);
    assert_eq!(app.patch_phrase_favorite_items(), vec!["c".to_string()]);
}

#[test]
fn handle_normal_o_and_o_insert_blank_line_and_enter_insert_mode() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.cursor = 1;
    app.list_state.select(Some(1));

    app.handle_normal(KeyCode::Char('o'));

    assert_eq!(
        app.lines,
        vec!["line 0".to_string(), "line 1".to_string(), String::new()]
    );
    assert_eq!(app.cursor, 2);
    assert_eq!(app.list_state.selected(), Some(2));
    assert!(matches!(app.mode, Mode::Insert));

    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.cursor = 1;
    app.list_state.select(Some(1));

    app.handle_normal(KeyCode::Char('O'));

    assert_eq!(
        app.lines,
        vec!["line 0".to_string(), String::new(), "line 1".to_string()]
    );
    assert_eq!(app.cursor, 1);
    assert_eq!(app.list_state.selected(), Some(1));
    assert!(matches!(app.mode, Mode::Insert));
}

#[test]
fn handle_normal_w_launches_daw() {
    let mut app = TuiApp::new_for_test(test_config());

    let result = app.handle_normal(KeyCode::Char('w'));

    assert!(matches!(result, NormalAction::LaunchDaw));
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn handle_normal_dd_yanks_current_line_and_keeps_notepad_non_empty() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![
        "line 0".to_string(),
        "line 1".to_string(),
        "line 2".to_string(),
    ];
    app.cursor = 1;
    app.list_state.select(Some(1));

    let result = app.handle_normal(KeyCode::Char('d'));

    assert!(matches!(result, NormalAction::Continue));
    assert!(app.normal_pending_delete);
    assert!(app.yank_buffer.is_none());

    let result = app.handle_normal(KeyCode::Char('d'));

    assert!(matches!(result, NormalAction::Continue));
    assert_eq!(app.lines, vec!["line 0".to_string(), "line 2".to_string()]);
    assert_eq!(app.cursor, 1);
    assert_eq!(app.list_state.selected(), Some(1));
    assert!(!app.normal_pending_delete);
    assert_eq!(app.yank_buffer.as_deref(), Some("line 1"));
}

#[test]
fn handle_normal_d_is_cleared_when_another_key_is_pressed() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.cursor = 0;
    app.list_state.select(Some(0));

    app.handle_normal(KeyCode::Char('d'));
    let result = app.handle_normal(KeyCode::Char('w'));

    assert!(matches!(result, NormalAction::LaunchDaw));
    assert_eq!(app.lines, vec!["line 0".to_string(), "line 1".to_string()]);
    assert!(!app.normal_pending_delete);
    assert!(app.yank_buffer.is_none());
}

#[test]
fn handle_normal_dd_on_single_line_replaces_with_empty() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["only".to_string()];

    app.handle_normal(KeyCode::Char('d'));
    app.handle_normal(KeyCode::Char('d'));

    assert_eq!(app.lines, vec![String::new()]);
    assert_eq!(app.cursor, 0);
    assert_eq!(app.yank_buffer.as_deref(), Some("only"));
}

#[test]
fn handle_normal_delete_yanks_current_line_and_keeps_notepad_non_empty() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![
        "line 0".to_string(),
        "line 1".to_string(),
        "line 2".to_string(),
    ];
    app.cursor = 1;
    app.list_state.select(Some(1));

    app.handle_normal(KeyCode::Delete);

    assert_eq!(app.lines, vec!["line 0".to_string(), "line 2".to_string()]);
    assert_eq!(app.cursor, 1);
    assert_eq!(app.list_state.selected(), Some(1));
    assert_eq!(app.yank_buffer.as_deref(), Some("line 1"));

    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["only".to_string()];

    app.handle_normal(KeyCode::Delete);

    assert_eq!(app.lines, vec![String::new()]);
    assert_eq!(app.cursor, 0);
    assert_eq!(app.yank_buffer.as_deref(), Some("only"));
}

#[test]
fn handle_normal_p_and_p_paste_yanked_line_below_or_above_cursor() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.cursor = 0;
    app.list_state.select(Some(0));
    app.yank_buffer = Some("yanked".to_string());

    app.handle_normal(KeyCode::Char('p'));

    assert_eq!(
        app.lines,
        vec![
            "line 0".to_string(),
            "yanked".to_string(),
            "line 1".to_string()
        ]
    );
    assert_eq!(app.cursor, 1);
    assert_eq!(app.list_state.selected(), Some(1));

    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["line 0".to_string(), "line 1".to_string()];
    app.cursor = 1;
    app.list_state.select(Some(1));
    app.yank_buffer = Some("yanked".to_string());

    app.handle_normal(KeyCode::Char('P'));

    assert_eq!(
        app.lines,
        vec![
            "line 0".to_string(),
            "yanked".to_string(),
            "line 1".to_string()
        ]
    );
    assert_eq!(app.cursor, 1);
    assert_eq!(app.list_state.selected(), Some(1));
}

#[test]
fn handle_normal_home_and_l_move_to_edges_and_play_destination_line() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![
        "line 0".to_string(),
        "line 1".to_string(),
        "line 2".to_string(),
        "line 3".to_string(),
    ];
    app.cursor = 1;
    app.list_state.select(Some(1));

    app.handle_normal(KeyCode::Char('L'));

    assert_eq!(app.cursor, 3);
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "line 3"
    ));

    app.handle_normal(KeyCode::Home);

    assert_eq!(app.cursor, 0);
    assert!(matches!(
        &*app.play_state.lock().unwrap(),
        PlayState::Running(msg) if msg == "line 0"
    ));
}
