use super::*;

/// `/loops/Pack/{Bass/{B.wav,a.wav}, Drums/Kick.wav}` を Pack まで展開した状態。
/// Bass と Drums は畳んだまま（絞り込み解除で折り畳みが戻ることを見るため）。
fn browser_with_pack_expanded() -> LoopBrowser {
    let mut browser = browser();
    browser.handle_key(KeyCode::Char('j'));
    browser.handle_key(KeyCode::Char('l'));
    assert_eq!(
        rows(&browser),
        vec![
            (0, "/loops".to_string(), false),
            (1, "Pack".to_string(), false),
            (2, "Bass".to_string(), false),
            (2, "Drums".to_string(), false),
        ]
    );
    browser
}

fn rows(browser: &LoopBrowser) -> Vec<(usize, String, bool)> {
    browser
        .visible
        .iter()
        .map(|node| (node.depth, node.name.clone(), node.is_wav))
        .collect()
}

fn snapshot(browser: &LoopBrowser) -> Vec<(usize, String, bool, bool, PathBuf)> {
    browser
        .visible
        .iter()
        .map(|node| {
            (
                node.depth,
                node.name.clone(),
                node.is_wav,
                node.expanded,
                node.path.clone(),
            )
        })
        .collect()
}

#[test]
fn a_wav_name_query_keeps_only_the_path_down_to_that_wav() {
    let mut browser = browser_with_pack_expanded();

    browser.set_filter_query("kick");

    assert!(browser.filter_active());
    assert_eq!(
        rows(&browser),
        vec![
            (0, "/loops".to_string(), false),
            (1, "Pack".to_string(), false),
            (2, "Drums".to_string(), false),
            (3, "Kick.wav".to_string(), true),
        ]
    );
}

#[test]
fn a_directory_query_keeps_the_whole_subtree_even_when_no_wav_name_matches() {
    let mut browser = browser_with_pack_expanded();

    browser.set_filter_query("drums");

    assert_eq!(
        rows(&browser),
        vec![
            (0, "/loops".to_string(), false),
            (1, "Pack".to_string(), false),
            (2, "Drums".to_string(), false),
            (3, "Kick.wav".to_string(), true),
        ]
    );
}

#[test]
fn terms_are_anded_and_matched_against_the_whole_relative_path() {
    let mut browser = browser_with_pack_expanded();

    browser.set_filter_query("pack kick");
    assert_eq!(
        rows(&browser).last().cloned(),
        Some((3, "Kick.wav".to_string(), true))
    );
    assert_eq!(browser.visible.len(), 4);

    browser.set_filter_query("bass kick");
    assert!(browser.visible.is_empty());
}

#[test]
fn the_query_is_case_insensitive_and_accepts_a_regular_expression() {
    let mut browser = browser_with_pack_expanded();

    browser.set_filter_query("KICK");
    assert_eq!(browser.visible.len(), 4);

    browser.set_filter_query("kick|a\\.wav");
    assert_eq!(
        rows(&browser),
        vec![
            (0, "/loops".to_string(), false),
            (1, "Pack".to_string(), false),
            (2, "Bass".to_string(), false),
            (3, "a.wav".to_string(), true),
            (2, "Drums".to_string(), false),
            (3, "Kick.wav".to_string(), true),
        ]
    );
}

/// `kick|` は妥当な正規表現で全件にマッチし、loop tree では
/// 「root 自身の相対パス（空文字列）にマッチする＝ツリー全部が残る」として出る。
#[test]
fn a_condition_that_matches_the_empty_string_keeps_the_whole_tree_expanded() {
    let mut browser = browser_with_pack_expanded();

    browser.set_filter_query("kick|");

    assert!(browser.filter_active());
    assert_eq!(
        rows(&browser),
        vec![
            (0, "/loops".to_string(), false),
            (1, "Pack".to_string(), false),
            (2, "Bass".to_string(), false),
            (3, "a.wav".to_string(), true),
            (3, "B.wav".to_string(), true),
            (2, "Drums".to_string(), false),
            (3, "Kick.wav".to_string(), true),
        ]
    );
}

#[test]
fn a_query_with_no_hit_leaves_the_tree_empty() {
    let mut browser = browser_with_pack_expanded();

    browser.set_filter_query("zzz");

    assert!(browser.visible.is_empty());
    assert_eq!(browser.cursor, 0);
    assert_eq!(browser.tree_scroll, 0);
}

#[test]
fn clearing_the_query_restores_the_collapse_state_from_before_the_filter() {
    let mut browser = browser_with_pack_expanded();
    let before = snapshot(&browser);

    browser.set_filter_query("kick");
    assert_ne!(snapshot(&browser), before);
    browser.set_filter_query("");

    assert!(!browser.filter_active());
    assert_eq!(browser.filter_query(), "");
    assert_eq!(snapshot(&browser), before);
}

#[test]
fn an_empty_query_never_touches_the_visible_rows() {
    let mut browser = browser_with_pack_expanded();
    let before = snapshot(&browser);

    browser.set_filter_query("   ");

    assert!(!browser.filter_active());
    assert_eq!(snapshot(&browser), before);
}

#[test]
fn an_invalid_condition_keeps_the_last_valid_result() {
    let mut browser = browser_with_pack_expanded();

    browser.set_filter_query("kick");
    let filtered = snapshot(&browser);
    browser.set_filter_query("kick(");

    assert_eq!(browser.filter_query(), "kick(");
    assert_eq!(snapshot(&browser), filtered);
}

#[test]
fn the_filter_narrows_within_favorites_only() {
    let mut browser = browser();
    let bass = LoopDirId::new(Path::new("/loops"), Path::new("Pack/Bass"));
    let drums = LoopDirId::new(Path::new("/loops"), Path::new("Pack/Drums"));
    browser.metadata.value.toggle_favorite(&bass);
    browser.metadata.value.toggle_favorite(&drums);
    browser.handle_key(KeyCode::Char('V'));
    assert!(browser.favorites_only);
    assert_eq!(browser.visible.len(), 2);

    browser.set_filter_query("kick");

    assert_eq!(browser.visible.len(), 2);
    assert!(browser.visible[0].path.ends_with("Drums"));
    assert!(browser.visible[1].path.ends_with("Kick.wav"));
}

#[test]
fn the_cursor_follows_the_selected_path_and_falls_back_to_the_first_row() {
    let mut browser = browser_with_pack_expanded();
    browser.set_filter_query("wav");
    let kick = browser
        .visible
        .iter()
        .position(|node| node.name == "Kick.wav")
        .unwrap();
    browser.cursor = kick;

    browser.set_filter_query("kick");
    assert!(browser.visible[browser.cursor].path.ends_with("Kick.wav"));

    browser.set_filter_query("bass");
    assert_eq!(browser.cursor, 0);
    assert_eq!(browser.visible[0].name, "/loops");
}

#[test]
fn the_filter_does_not_mutate_the_saved_expanded_set() {
    let mut browser = browser_with_pack_expanded();
    let expanded_before = browser.expanded.clone();

    browser.set_filter_query("kick");

    assert_eq!(browser.expanded, expanded_before);
}

fn ctrl(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::CONTROL)
}

fn type_query(browser: &mut LoopBrowser, text: &str) {
    for character in text.chars() {
        browser.handle_key(KeyCode::Char(character));
    }
}

#[test]
fn slash_then_typing_then_enter_commits_the_query() {
    let mut browser = browser_with_pack_expanded();

    browser.handle_key(KeyCode::Char('/'));
    assert!(browser.filter_input_active());

    type_query(&mut browser, "kick");
    assert_eq!(browser.filter_query(), "kick");

    assert!(matches!(
        browser.handle_key(KeyCode::Enter),
        LoopBrowserAction::Continue
    ));
    assert!(!browser.filter_input_active());
    assert!(browser.filter_active());
    assert_eq!(
        rows(&browser),
        vec![
            (0, "/loops".to_string(), false),
            (1, "Pack".to_string(), false),
            (2, "Drums".to_string(), false),
            (3, "Kick.wav".to_string(), true),
        ]
    );
}

#[test]
fn ctrl_m_commits_like_enter() {
    let mut browser = browser_with_pack_expanded();

    browser.handle_key(KeyCode::Char('/'));
    type_query(&mut browser, "kick");
    browser.handle_key_event(ctrl(KeyCode::Char('m')));

    assert!(!browser.filter_input_active());
    assert_eq!(browser.filter_query(), "kick");
}

#[test]
fn slash_seeds_the_input_with_the_committed_query() {
    let mut browser = browser_with_pack_expanded();
    browser.handle_key(KeyCode::Char('/'));
    type_query(&mut browser, "kick");
    browser.handle_key(KeyCode::Enter);

    browser.handle_key(KeyCode::Char('/'));
    browser.handle_key(KeyCode::Enter);

    assert_eq!(browser.filter_query(), "kick");
    assert!(browser.filter_active());
}

#[test]
fn escape_returns_to_the_query_and_selection_from_before_the_input() {
    let mut browser = browser_with_pack_expanded();
    browser.set_filter_query("bass");
    let rows_before = rows(&browser);
    browser.cursor = browser.visible.len() - 1;
    let path_before = browser.visible[browser.cursor].path.clone();

    browser.handle_key(KeyCode::Char('/'));
    type_query(&mut browser, "kick");
    assert_eq!(browser.filter_query(), "basskick");

    assert!(matches!(
        browser.handle_key(KeyCode::Esc),
        LoopBrowserAction::Continue
    ));

    assert!(!browser.filter_input_active());
    assert_eq!(browser.filter_query(), "bass");
    assert_eq!(rows(&browser), rows_before);
    assert_eq!(browser.visible[browser.cursor].path, path_before);
}

#[test]
fn keys_that_are_commands_outside_the_input_are_typed_as_characters() {
    let mut browser = browser_with_pack_expanded();
    browser.handle_key(KeyCode::Char('/'));

    for character in "jrq?p".chars() {
        assert!(
            matches!(
                browser.handle_key(KeyCode::Char(character)),
                LoopBrowserAction::Continue
            ),
            "{character} escaped the filter input"
        );
    }

    assert!(browser.filter_input_active());
    assert!(browser.help_overlay.is_none());
    assert!(!browser.playback_paused);
    assert_eq!(browser.filter_query(), "jrq?p");
}

#[test]
fn digits_and_tab_do_not_reach_the_navigation_count_or_the_pane_focus() {
    let mut browser = browser_with_pack_expanded();
    browser.handle_key(KeyCode::Char('/'));

    type_query(&mut browser, "12");
    browser.handle_key(KeyCode::Tab);

    assert_eq!(browser.focus, LoopBrowserPane::Tree);
    assert!(browser.filter_input_active());
    browser.handle_key(KeyCode::Enter);
    assert!(browser.filter_query().starts_with("12"));
}

#[test]
fn clearing_the_input_and_committing_releases_the_filter() {
    let mut browser = browser_with_pack_expanded();
    let before = snapshot(&browser);
    browser.handle_key(KeyCode::Char('/'));
    type_query(&mut browser, "kick");
    assert!(browser.filter_active());

    // 行頭へ移動してから行末まで削除する（textarea の標準 keybind）。
    browser.handle_key_event(ctrl(KeyCode::Char('a')));
    browser.handle_key_event(ctrl(KeyCode::Char('k')));
    assert_eq!(browser.filter_query(), "");

    browser.handle_key(KeyCode::Enter);
    assert!(!browser.filter_input_active());
    assert!(!browser.filter_active());
    assert_eq!(snapshot(&browser), before);
}

/// 絞り込みは永続化しない（セッション DTO にもメタデータにも無い）。
/// 起動直後の状態がそれをそのまま示す。
#[test]
fn a_freshly_built_browser_has_no_filter() {
    let browser = browser();

    assert_eq!(browser.filter_query(), "");
    assert!(!browser.filter_active());
    assert!(!browser.filter_input_active());
}
