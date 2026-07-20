use super::*;
use crate::loop_library::{LoopIndex, LoopRootIndex};

fn browser() -> LoopBrowser {
    LoopBrowser::from_index(
        LoopIndex {
            version: 1,
            roots: vec![LoopRootIndex {
                path: "/loops".to_string(),
                wav_files: vec![
                    "Pack/Bass/B.wav".to_string(),
                    "Pack/Bass/a.wav".to_string(),
                    "Pack/Drums/Kick.wav".to_string(),
                ],
            }],
        },
        &crate::config::default_loop_categories(),
        LoopBrowserMetadata::default(),
        None,
        true,
        None,
    )
}

fn browser_with_direct_wavs(count: usize) -> LoopBrowser {
    LoopBrowser::from_index(
        LoopIndex {
            version: 1,
            roots: vec![LoopRootIndex {
                path: "/loops".to_string(),
                wav_files: (0..count).map(|index| format!("{index:02}.wav")).collect(),
            }],
        },
        &crate::config::default_loop_categories(),
        LoopBrowserMetadata::default(),
        None,
        true,
        None,
    )
}

fn select_bass_wav(browser: &mut LoopBrowser) {
    browser.handle_key(KeyCode::Char('j'));
    browser.handle_key(KeyCode::Char('l'));
    browser.handle_key(KeyCode::Char('j'));
    browser.handle_key(KeyCode::Char('l'));
    browser.handle_key(KeyCode::Char('j'));
}

#[test]
fn root_is_expanded_and_directories_sort_before_wavs() {
    let browser = browser();
    assert_eq!(browser.visible.len(), 2);
    assert_eq!(browser.visible[0].name, "/loops");
    assert_eq!(browser.visible[1].name, "Pack");
}

#[test]
fn hjkl_expand_navigate_play_and_select_parent() {
    let mut browser = browser();
    assert!(matches!(
        browser.handle_key(KeyCode::Char('j')),
        LoopBrowserAction::Continue
    ));
    browser.handle_key(KeyCode::Char('l'));
    assert_eq!(browser.visible.len(), 4);
    browser.handle_key(KeyCode::Char('j'));
    browser.handle_key(KeyCode::Char('l'));
    assert_eq!(browser.visible[3].name, "a.wav");
    assert!(matches!(
        browser.handle_key(KeyCode::Char('j')),
        LoopBrowserAction::Preview(path) if path.ends_with("a.wav")
    ));
    browser.handle_key(KeyCode::Char('h'));
    assert_eq!(browser.visible[browser.cursor].name, "Bass");
}

#[test]
fn page_keys_move_ten_rows_and_preview_only_the_destination() {
    let mut browser = browser_with_direct_wavs(15);

    assert!(matches!(
        browser.handle_key(KeyCode::PageDown),
        LoopBrowserAction::Preview(_)
    ));
    assert_eq!(browser.cursor, 10);
    assert!(matches!(
        browser.handle_key(KeyCode::PageUp),
        LoopBrowserAction::Continue
    ));
    assert_eq!(browser.cursor, 0);
}

#[test]
fn tree_hjkl_accepts_vim_style_numeric_prefixes() {
    let mut browser = browser_with_direct_wavs(15);

    browser.handle_key(KeyCode::Char('1'));
    browser.handle_key(KeyCode::Char('0'));
    assert!(matches!(
        browser.handle_key(KeyCode::Char('j')),
        LoopBrowserAction::Preview(_)
    ));
    assert_eq!(browser.cursor, 10);

    browser.handle_key(KeyCode::Char('2'));
    browser.handle_key(KeyCode::Char('k'));
    assert_eq!(browser.cursor, 8);

    browser.handle_key(KeyCode::Char('3'));
    browser.handle_key(KeyCode::Down);
    assert_eq!(browser.cursor, 9);
    assert_eq!(browser.navigation_count.value(), None);
}

#[test]
fn prefixed_tree_h_repeats_parent_and_collapse_operations() {
    let mut browser = browser();
    select_bass_wav(&mut browser);

    browser.handle_key(KeyCode::Char('2'));
    browser.handle_key(KeyCode::Char('h'));

    assert_eq!(browser.visible[browser.cursor].name, "Bass");
    assert!(!browser.visible.iter().any(|node| node.name == "a.wav"));
}

#[test]
fn favorite_from_wav_targets_parent_and_removal_shows_expiring_notice() {
    let mut browser = browser();
    select_bass_wav(&mut browser);
    let expected = LoopDirId::new(Path::new("/loops"), Path::new("Pack/Bass"));

    browser.handle_key(KeyCode::Char('v'));
    assert!(browser.metadata.is_favorite(&expected));
    assert!(browser
        .visible
        .iter()
        .any(|node| node.name == "Bass" && node.favorite));
    assert!(browser.notice.is_none());

    browser.handle_key(KeyCode::Char('v'));
    assert!(!browser.metadata.is_favorite(&expected));
    assert_eq!(
        browser.active_notice().map(|notice| notice.text.as_str()),
        Some("お気に入りdirを解除しました")
    );
    browser.notice.as_mut().unwrap().expires_at = Instant::now();
    assert!(browser.active_notice().is_none());
}

#[test]
fn favorites_only_lists_each_favorite_as_a_top_level_browsable_dir() {
    let mut browser = browser();
    let pack = LoopDirId::new(Path::new("/loops"), Path::new("Pack"));
    let bass = LoopDirId::new(Path::new("/loops"), Path::new("Pack/Bass"));
    browser.metadata.toggle_favorite(&pack);
    browser.metadata.toggle_favorite(&bass);
    browser.rebuild_visible(None);

    browser.handle_key(KeyCode::Char('V'));
    assert!(browser.favorites_only);
    assert_eq!(browser.visible.len(), 2);
    assert!(browser.visible.iter().all(|node| node.depth == 0));
    assert!(browser.visible[0].path.ends_with("Pack"));
    assert!(browser.visible[1].path.ends_with("Pack/Bass"));

    browser.handle_key(KeyCode::Char('l'));
    assert!(browser.visible.iter().any(|node| node.name == "Bass"));
    browser.handle_key(KeyCode::Char('V'));
    assert!(!browser.favorites_only);
    assert_eq!(browser.visible[0].name, "/loops");
}

#[test]
fn favorites_only_handles_an_empty_favorite_set() {
    let mut browser = browser();

    browser.handle_key(KeyCode::Char('V'));

    assert!(browser.favorites_only);
    assert!(browser.visible.is_empty());
    assert_eq!(browser.list_state.selected(), None);
}

#[test]
fn favorite_save_failure_rolls_back_in_memory_change() {
    let mut browser = browser();
    browser.cursor = 1;
    let temp = std::env::temp_dir().join(format!(
        "cmrt-loop-browser-blocked-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&temp).unwrap();
    browser.metadata_path = Some(temp.clone());

    browser.handle_key(KeyCode::Char('v'));

    assert!(browser.metadata.favorite_dirs.is_empty());
    assert!(browser
        .metadata_error
        .as_deref()
        .is_some_and(|error| error.contains("お気に入りを保存できません")));
    let _ = std::fs::remove_dir_all(temp);
}

#[test]
fn category_overlay_assigns_and_toggles_category_for_wav_parent() {
    let mut browser = browser();
    select_bass_wav(&mut browser);
    let expected = LoopDirId::new(Path::new("/loops"), Path::new("Pack/Bass"));

    browser.handle_key(KeyCode::Char('t'));
    assert!(browser
        .category_overlay
        .as_ref()
        .is_some_and(|target| target.matches(&expected)));
    browser.handle_key(KeyCode::Char('b'));
    assert_eq!(browser.metadata.category_for(&expected), Some("bass"));
    assert!(browser
        .visible
        .iter()
        .any(|node| node.name == "Bass" && node.category.as_deref() == Some("bass")));

    browser.handle_key(KeyCode::Char('t'));
    browser.handle_key(KeyCode::Char('b'));
    assert_eq!(browser.metadata.category_for(&expected), None);
}

#[test]
fn category_overlay_escape_keeps_assignment_unchanged() {
    let mut browser = browser();
    browser.cursor = 1;
    browser.handle_key(KeyCode::Char('t'));
    assert!(browser.category_overlay.is_some());
    browser.handle_key(KeyCode::Esc);
    assert!(browser.category_overlay.is_none());
    assert!(browser.metadata.category_assignments.is_empty());
}

#[test]
fn shift_note_assigns_and_removes_pad_while_lowercase_triggers_it() {
    let mut browser = browser();
    select_bass_wav(&mut browser);

    browser.handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));

    assert!(browser
        .metadata
        .pad('c')
        .is_some_and(|wav| wav.path().ends_with("a.wav")));
    assert!(matches!(
        browser.handle_key(KeyCode::Char('c')),
        LoopBrowserAction::Trigger { pad: 'c', path } if path.ends_with("a.wav")
    ));

    browser.handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    assert!(browser.metadata.pad('c').is_none());
    assert_eq!(
        browser.active_notice().map(|notice| notice.text.as_str()),
        Some("WAV pad C を解除しました")
    );
}

#[test]
fn favorites_only_shift_a_saves_only_pad_metadata() {
    let mut browser = browser();
    let favorite = LoopDirId::new(Path::new("/loops"), Path::new("Pack/Bass"));
    browser.metadata.toggle_favorite(&favorite);
    browser.rebuild_visible(None);
    let dir = std::env::temp_dir().join(format!(
        "cmrt-loop-browser-favorites-pad-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let path = dir.join("loop_browser.toml");
    let blocked_track_grid_path = dir.join("blocked-track-grid");
    std::fs::create_dir_all(&blocked_track_grid_path).unwrap();
    browser.metadata_path = Some(path.clone());
    browser.track_grid_path = Some(blocked_track_grid_path);

    browser.handle_key(KeyCode::Char('V'));
    browser.handle_key(KeyCode::Char('l'));
    browser.handle_key(KeyCode::Char('j'));
    browser.handle_key_event(KeyEvent::new(KeyCode::Char('A'), KeyModifiers::SHIFT));

    assert!(browser.metadata_error.is_none());
    assert!(browser.track_grid_error.is_none());
    assert!(browser
        .metadata
        .pad('a')
        .is_some_and(|wav| wav.path().ends_with("a.wav")));
    assert_eq!(
        LoopBrowserMetadata::load_from(&path).unwrap(),
        browser.metadata
    );
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn track_pane_toggles_one_wav_per_cell_and_auto_extends_right_and_down() {
    let mut browser = browser();
    select_bass_wav(&mut browser);
    browser.handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    browser.handle_key(KeyCode::Tab);
    assert_eq!(browser.focus, LoopBrowserPane::Tracks);

    assert!(matches!(
        browser.handle_key(KeyCode::Char('c')),
        LoopBrowserAction::GridChanged { pad: 'c', audition, grid }
            if audition.ends_with("a.wav") && grid[0][0].as_ref().is_some_and(|path| path.ends_with("a.wav"))
    ));
    browser.handle_key(KeyCode::Char('l'));
    browser.handle_key(KeyCode::Char('j'));
    assert_eq!((browser.track_cursor, browser.measure_cursor), (1, 1));
    assert_eq!(browser.track_grid.len(), 2);
    assert!(browser.track_grid.iter().all(|track| track.len() == 2));

    browser.handle_key(KeyCode::Char('h'));
    browser.handle_key(KeyCode::Char('k'));
    assert_eq!((browser.track_cursor, browser.measure_cursor), (0, 0));
    assert!(matches!(
        browser.handle_key(KeyCode::Char('c')),
        LoopBrowserAction::GridChanged { grid, .. } if grid[0][0].is_none()
    ));
}

#[test]
fn track_hjkl_prefix_moves_and_extends_by_the_requested_count() {
    let mut browser = browser();
    browser.focus = LoopBrowserPane::Tracks;

    browser.handle_key(KeyCode::Char('3'));
    browser.handle_key(KeyCode::Char('l'));
    assert_eq!(browser.measure_cursor, 3);
    assert!(browser.track_grid.iter().all(|track| track.len() == 4));

    browser.handle_key(KeyCode::Char('2'));
    browser.handle_key(KeyCode::Char('j'));
    assert_eq!(browser.track_cursor, 2);
    assert_eq!(browser.track_grid.len(), 3);

    browser.handle_key(KeyCode::Char('2'));
    browser.handle_key(KeyCode::Char('h'));
    browser.handle_key(KeyCode::Char('2'));
    browser.handle_key(KeyCode::Char('k'));
    assert_eq!((browser.track_cursor, browser.measure_cursor), (0, 1));
}

#[test]
fn replacing_pad_does_not_change_existing_track_cell() {
    let mut browser = browser();
    select_bass_wav(&mut browser);
    browser.handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    browser.handle_key(KeyCode::Tab);
    browser.handle_key(KeyCode::Char('c'));
    browser.handle_key(KeyCode::Tab);
    browser.handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    assert!(browser.notice.is_some());
    browser.handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    assert!(browser.notice.is_none());
    browser.handle_key(KeyCode::Char('j'));
    browser.handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));

    assert!(browser.notice.is_none());
    assert!(browser
        .metadata
        .pad('c')
        .is_some_and(|wav| wav.path().ends_with("B.wav")));
    assert!(browser.track_grid[0][0]
        .as_ref()
        .is_some_and(|wav| wav.path().ends_with("a.wav")));
}
