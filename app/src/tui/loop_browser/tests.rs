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
        LoopBrowserAction::Play(path) if path.ends_with("a.wav")
    ));
    browser.handle_key(KeyCode::Char('h'));
    assert_eq!(browser.visible[browser.cursor].name, "Bass");
}

#[test]
fn favorite_from_wav_targets_parent_and_removal_shows_expiring_notice() {
    let mut browser = browser();
    select_bass_wav(&mut browser);
    let expected = LoopDirId::new(Path::new("/loops"), Path::new("Pack/Bass"));

    browser.handle_key(KeyCode::Char('f'));
    assert!(browser.metadata.is_favorite(&expected));
    assert!(browser
        .visible
        .iter()
        .any(|node| node.name == "Bass" && node.favorite));
    assert!(browser.notice.is_none());

    browser.handle_key(KeyCode::Char('f'));
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

    browser.handle_key(KeyCode::Char('F'));
    assert!(browser.favorites_only);
    assert_eq!(browser.visible.len(), 2);
    assert!(browser.visible.iter().all(|node| node.depth == 0));
    assert!(browser.visible[0].path.ends_with("Pack"));
    assert!(browser.visible[1].path.ends_with("Pack/Bass"));

    browser.handle_key(KeyCode::Char('l'));
    assert!(browser.visible.iter().any(|node| node.name == "Bass"));
    browser.handle_key(KeyCode::Char('F'));
    assert!(!browser.favorites_only);
    assert_eq!(browser.visible[0].name, "/loops");
}

#[test]
fn favorites_only_handles_an_empty_favorite_set() {
    let mut browser = browser();

    browser.handle_key(KeyCode::Char('F'));

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

    browser.handle_key(KeyCode::Char('f'));

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

    browser.handle_key(KeyCode::Char('c'));
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

    browser.handle_key(KeyCode::Char('c'));
    browser.handle_key(KeyCode::Char('b'));
    assert_eq!(browser.metadata.category_for(&expected), None);
}

#[test]
fn category_overlay_escape_keeps_assignment_unchanged() {
    let mut browser = browser();
    browser.cursor = 1;
    browser.handle_key(KeyCode::Char('c'));
    assert!(browser.category_overlay.is_some());
    browser.handle_key(KeyCode::Esc);
    assert!(browser.category_overlay.is_none());
    assert!(browser.metadata.category_assignments.is_empty());
}
