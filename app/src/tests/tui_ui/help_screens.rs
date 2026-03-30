use super::*;

#[test]
fn patch_phrase_screen_shows_search_prompt() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::PatchPhrase;
    app.patch_phrase_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_query = "jk".to_string();
    app.patch_phrase_filter_active = true;

    let lines = render_lines(&mut app, 120, 12).join("\n");

    assert!(lines.contains("/ jk"));
}

#[test]
fn normal_help_screen_mentions_ctrl_clipboard_shortcuts_without_overlay_keybinds() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::Help;
    app.help_origin = Mode::Normal;

    let lines = render_lines(&mut app, 120, 60);
    let screen = lines.join("\n");
    let normalized_lines: Vec<String> = lines.iter().map(|line| line.replace(' ', "")).collect();

    assert!(screen.contains("[HELP] notepad mode"));
    assert!(normalized_lines
        .iter()
        .any(|line| line.contains("Ctrl+C:コピー")));
    assert!(normalized_lines
        .iter()
        .any(|line| line.contains("K/?:ヘルプ(このページ)")));
    assert!(normalized_lines
        .iter()
        .any(|line| line.contains("Ctrl+X:カット")));
    assert!(normalized_lines
        .iter()
        .any(|line| line.contains("Ctrl+V:ペースト")));
    assert!(normalized_lines
        .iter()
        .any(|line| line.contains("Shift+H:patchhistory")));
    assert!(normalized_lines
        .iter()
        .any(|line| line.contains("g:generateを上に挿入して再生")));
    assert!(normalized_lines
        .iter()
        .any(|line| line.contains("dd/Del:削除（ヤンク）p/P:下貼付/上貼付")));
    assert!(normalized_lines
        .iter()
        .any(|line| line.contains("w:DAWモード")));
    assert!(!normalized_lines
        .iter()
        .any(|line| line.contains("Ctrl+F:現在音色とMMLをFavorites追加")));
    assert!(!normalized_lines
        .iter()
        .any(|line| line.contains("h/l・←/→:ペイン切替")));
    assert!(!normalized_lines
        .iter()
        .any(|line| line.contains("Ctrl+C:強制終了")));
}

#[test]
fn patch_select_help_screen_shows_patch_select_shortcuts() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string()];
    app.patch_all = vec![("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string())];
    app.patch_filtered = vec!["Pads/Pad 1.fxp".to_string()];
    app.patch_list_state.select(Some(0));
    app.mode = Mode::Help;
    app.help_origin = Mode::PatchSelect;

    let lines = render_lines(&mut app, 120, 32);
    let normalized_screen = lines.join("\n").replace([' ', '\n'], "");

    assert!(normalized_screen.contains("音色選択モード"));
    assert!(normalized_screen.contains("/:patchname絞り込み開始"));
    assert!(normalized_screen.contains("n/p/t:notepadhistory/patchhistory/音色選択"));
    assert!(normalized_screen.contains("f:現在音色とMMLをFavorites追加"));
    assert!(normalized_screen.contains("h/l・←/→:ペイン切替して再生"));
    assert!(!normalized_screen.contains("Ctrl+C:コピー"));
}

#[test]
fn patch_select_help_screen_keeps_patch_select_base_title_and_keybinds() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string()];
    app.patch_all = vec![("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string())];
    app.patch_filtered = vec!["Pads/Pad 1.fxp".to_string()];
    app.patch_list_state.select(Some(0));
    app.mode = Mode::Help;
    app.help_origin = Mode::PatchSelect;

    let screen = render_lines(&mut app, 120, 32).join("\n");

    assert!(screen.contains("[PATCH SELECT] notepad mode"));
    assert!(!screen.contains("[HELP] notepad mode"));
}

#[test]
fn notepad_history_help_screen_shows_history_shortcuts() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad = PatchPhraseState {
        history: vec!["l8cdef".to_string()],
        favorites: vec!["o5g".to_string()],
    };
    app.mode = Mode::Help;
    app.help_origin = Mode::NotepadHistory;
    app.notepad_focus = crate::tui::PatchPhrasePane::History;
    app.notepad_history_state.select(Some(0));
    app.notepad_favorites_state.select(Some(0));

    let lines = render_lines(&mut app, 120, 32);
    let normalized_screen = lines.join("\n").replace([' ', '\n'], "");

    assert!(normalized_screen.contains("notepadhistory画面"));
    assert!(normalized_screen.contains("/の後に文字入力:フィルタ(Space=AND条件)"));
    assert!(normalized_screen.contains("n/p/t:notepadhistory/patchhistory/音色選択"));
    assert!(normalized_screen.contains("h/l・←/→:ペイン切替"));
    assert!(normalized_screen.contains("dd:Favorites行を削除してHistory先頭へ移動"));
    assert!(!normalized_screen.contains("Ctrl+C:コピー"));
}

#[test]
fn notepad_history_help_screen_keeps_history_base_title_and_keybinds() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad = PatchPhraseState {
        history: vec!["l8cdef".to_string()],
        favorites: vec!["o5g".to_string()],
    };
    app.mode = Mode::Help;
    app.help_origin = Mode::NotepadHistory;
    app.notepad_focus = crate::tui::PatchPhrasePane::History;
    app.notepad_history_state.select(Some(0));
    app.notepad_favorites_state.select(Some(0));

    let screen = render_lines(&mut app, 120, 32).join("\n");

    assert!(screen.contains("[HISTORY] notepad mode"));
    assert!(!screen.contains("[HELP] notepad mode"));
}

#[test]
fn patch_phrase_help_screen_shows_patch_phrase_shortcuts() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );
    app.mode = Mode::Help;
    app.help_origin = Mode::PatchPhrase;

    let lines = render_lines(&mut app, 120, 32);
    let normalized_screen = lines.join("\n").replace([' ', '\n'], "");

    assert!(normalized_screen.contains("patchphrase画面"));
    assert!(normalized_screen.contains("/の後に文字入力:フィルタ(Space=AND条件)"));
    assert!(normalized_screen.contains("n/p/t:notepadhistory/patchhistory/音色選択"));
    assert!(normalized_screen.contains("h/l・←/→:ペイン切替して再生"));
    assert!(normalized_screen.contains("Space:現在行を再生"));
    assert!(normalized_screen.contains("f:現在行をお気に入りに追加"));
    assert!(!normalized_screen.contains("Ctrl+C:コピー"));
}
