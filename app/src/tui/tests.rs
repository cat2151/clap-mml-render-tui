use super::*;
use crossterm::event::KeyCode;
use std::sync::atomic::{AtomicUsize, Ordering};

static NEXT_TEST_ID: AtomicUsize = AtomicUsize::new(0);

fn make_patches(items: &[&str]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|&s| (s.to_string(), s.to_lowercase()))
        .collect()
}

#[test]
fn filter_patches_empty_query_returns_all() {
    let all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    let result = filter_patches(&all, "");
    assert_eq!(result, vec!["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
}

#[test]
fn filter_patches_single_term_matches_substring() {
    let all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    let result = filter_patches(&all, "pad");
    assert_eq!(result, vec!["Pads/Pad 1.fxp"]);
}

#[test]
fn filter_patches_case_insensitive() {
    let all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    let result = filter_patches(&all, "PAD");
    assert_eq!(result, vec!["Pads/Pad 1.fxp"]);
}

#[test]
fn filter_patches_multiple_terms_act_as_and() {
    let all = make_patches(&["Pads/Soft Pad.fxp", "Pads/Hard Pad.fxp", "Leads/Lead 1.fxp"]);
    let result = filter_patches(&all, "pad soft");
    assert_eq!(result, vec!["Pads/Soft Pad.fxp"]);
}

#[test]
fn filter_patches_no_match_returns_empty() {
    let all = make_patches(&["Pads/Pad 1.fxp"]);
    let result = filter_patches(&all, "xyznomatch");
    assert!(result.is_empty());
}

#[test]
fn filter_patches_whitespace_only_query_returns_all() {
    let all = make_patches(&["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
    // split_whitespace で空のイテレータになり、全件返す
    let result = filter_patches(&all, "   ");
    assert_eq!(result, vec!["Pads/Pad 1.fxp", "Leads/Lead 1.fxp"]);
}

#[test]
fn filter_patches_empty_list_returns_empty() {
    let all: Vec<(String, String)> = vec![];
    let result = filter_patches(&all, "pad");
    assert!(result.is_empty());
}

// --- audio cache helper tests ---

#[test]
fn resolve_cached_samples_returns_samples_on_cache_hit() {
    let mut cache = HashMap::new();
    cache.insert("cde".to_string(), vec![0.5f32, 0.6]);
    let result = resolve_cached_samples(Some(&cache), "cde");
    assert_eq!(result, Some(vec![0.5f32, 0.6]));
}

#[test]
fn resolve_cached_samples_returns_none_on_cache_miss() {
    let cache: HashMap<String, Vec<f32>> = HashMap::new();
    let result = resolve_cached_samples(Some(&cache), "cde");
    assert!(result.is_none());
}

#[test]
fn resolve_cached_samples_returns_none_without_cache_reference() {
    let mut cache = HashMap::new();
    cache.insert("cde".to_string(), vec![0.0f32, 1.0]);
    let result = resolve_cached_samples(None, "cde");
    assert!(result.is_none());
}

#[test]
fn try_insert_cache_does_nothing_when_random_patch_true() {
    let mut cache = HashMap::new();
    try_insert_cache(&mut cache, "cde".to_string(), vec![1.0f32], true);
    assert!(cache.is_empty());
}

#[test]
fn try_insert_cache_inserts_when_random_patch_false() {
    let mut cache = HashMap::new();
    try_insert_cache(&mut cache, "cde".to_string(), vec![1.0f32], false);
    assert!(cache.contains_key("cde"));
}

#[test]
fn try_insert_cache_clears_and_inserts_when_full() {
    let mut cache = HashMap::new();
    // AUDIO_CACHE_MAX_ENTRIES まで埋める
    for i in 0..AUDIO_CACHE_MAX_ENTRIES {
        cache.insert(format!("mml_{}", i), vec![]);
    }
    assert_eq!(cache.len(), AUDIO_CACHE_MAX_ENTRIES);

    // 新しいキーの挿入でクリアが起きる
    try_insert_cache(&mut cache, "new_mml".to_string(), vec![0.1f32], false);
    // クリア後に1エントリだけ残る
    assert_eq!(cache.len(), 1);
    assert!(cache.contains_key("new_mml"));
}

#[test]
fn try_insert_cache_updates_existing_key_when_full() {
    let mut cache = HashMap::new();
    // "cde" を含めてちょうど AUDIO_CACHE_MAX_ENTRIES 件になるよう埋める
    for i in 0..(AUDIO_CACHE_MAX_ENTRIES - 1) {
        cache.insert(format!("mml_{}", i), vec![]);
    }
    cache.insert("cde".to_string(), vec![]);
    assert_eq!(cache.len(), AUDIO_CACHE_MAX_ENTRIES);

    // 上限ちょうどの状態で既存キーを更新してもクリアは発生しない
    try_insert_cache(&mut cache, "cde".to_string(), vec![0.9f32], false);
    assert_eq!(cache.len(), AUDIO_CACHE_MAX_ENTRIES);
    assert_eq!(cache["cde"], vec![0.9f32]);
}

fn test_config() -> crate::config::Config {
    crate::config::Config {
        plugin_path: "/tmp/Surge XT.clap".to_string(),
        input_midi: "input.mid".to_string(),
        output_midi: "output.mid".to_string(),
        output_wav: "output.wav".to_string(),
        sample_rate: 44_100.0,
        buffer_size: 512,
        patch_path: None,
        patches_dir: Some("/tmp/patches".to_string()),
        daw_tracks: 9,
        daw_measures: 8,
    }
}

#[test]
fn handle_normal_r_toggles_random_timbre_mode() {
    let mut app = TuiApp::new_for_test(test_config());
    assert!(!app.random_timbre_enabled);

    let result = app.handle_normal(KeyCode::Char('r'));
    assert!(matches!(result, NormalAction::Continue));
    assert!(app.random_timbre_enabled);

    app.handle_normal(KeyCode::Char('r'));
    assert!(!app.random_timbre_enabled);
}

#[test]
fn handle_normal_t_shows_error_when_random_timbre_enabled() {
    let mut app = TuiApp::new_for_test(test_config());
    app.random_timbre_enabled = true;

    app.handle_normal(KeyCode::Char('t'));

    assert!(
        matches!(&*app.play_state.lock().unwrap(), PlayState::Err(msg) if msg == "ランダム音色モードでは音色選択は使えません")
    );
    assert!(matches!(app.mode, Mode::Normal));
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
fn save_history_state_persists_tui_cursor_lines_and_mode_flag() {
    let unique = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_tui_save_history_state_{}_{}",
        std::process::id(),
        unique
    ));
    std::fs::remove_dir_all(&tmp).ok();
    let _env_guards = crate::test_utils::set_data_local_dir_envs(&tmp);

    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string(), "def".to_string(), "ghi".to_string()];
    app.cursor = 2;
    app.is_daw_mode = true;

    app.save_history_state();

    let history_path = crate::test_utils::session_state_path_for_test()
        .expect("data local dir should resolve in isolated TUI history test");
    assert!(
        history_path.exists(),
        "expected isolated history file to be created at {}",
        history_path.display()
    );
    let saved = crate::history::load_session_state();
    assert_eq!(saved.cursor, 2);
    assert_eq!(saved.lines, app.lines);
    assert!(saved.is_daw_mode);

    std::fs::remove_dir_all(&tmp).ok();
}
