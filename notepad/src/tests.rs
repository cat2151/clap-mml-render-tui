pub(super) use super::*;
pub(super) use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
pub(super) use ratatui_textarea::{CursorMove, TextArea};
pub(super) use std::sync::atomic::{AtomicUsize, Ordering};

mod filter_cache;
mod insert_mode;
mod normal_mode;
mod notepad_history;
mod notepad_history_persistence;
mod patch_phrase;
mod patch_phrase_history;
mod patch_select;
mod patch_select_favorites;

static NEXT_TEST_ID: AtomicUsize = AtomicUsize::new(0);

fn make_patches(items: &[&str]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|&s| (s.to_string(), s.to_lowercase()))
        .collect()
}

pub(crate) fn test_config() -> Config {
    Config {
        plugin_path: "/tmp/Surge XT.clap".to_string(),
        input_midi: "input.mid".to_string(),
        output_midi: "output.mid".to_string(),
        output_wav: "output.wav".to_string(),
        sample_rate: 44_100.0,
        buffer_size: 512,
        patches_dirs: Some(vec!["/tmp/patches".to_string()]),
        loop_dirs: Vec::new(),
        loop_categories: cmrt_runtime::default_loop_categories(),
        offline_render_workers: cmrt_runtime::DEFAULT_OFFLINE_RENDER_WORKERS,
        offline_render_server_workers: cmrt_runtime::DEFAULT_OFFLINE_RENDER_SERVER_WORKERS,
        offline_render_backend: cmrt_runtime::OfflineRenderBackend::InProcess,
        offline_render_server_port: cmrt_runtime::DEFAULT_OFFLINE_RENDER_SERVER_PORT,
        offline_render_server_command: String::new(),
        realtime_audio_backend: cmrt_runtime::RealtimeAudioBackend::InProcess,
        realtime_play_server_port: cmrt_runtime::DEFAULT_REALTIME_PLAY_SERVER_PORT,
        realtime_play_server_command: String::new(),
        realtime_play_server_prewarm: false,
        autoplay_on_startup: true,
        voicing_shared_source: String::new(),
        voicing_override_source: String::new(),
        chord_progression_source: String::new(),
        chord_patch_categories: Vec::new(),
    }
}

/// 注入された sink が受け取った行。sink は `fn` ポインタなのでキャプチャできず、static で受ける。
static CAPTURED: std::sync::Mutex<Vec<String>> = std::sync::Mutex::new(Vec::new());

fn capture(line: &str) {
    CAPTURED.lock().unwrap().push(line.to_string());
}

/// sink 注入漏れは「ログが黙って消える」形で失敗するため、経路そのものを固定しておく。
#[test]
fn injected_sink_receives_prefixed_lines() {
    set_log_sink(capture);
    CAPTURED.lock().unwrap().clear();

    NotepadScreen::log_notepad_event("play request session=1 cache=hit");

    assert_eq!(
        CAPTURED.lock().unwrap().as_slice(),
        ["notepad: play request session=1 cache=hit".to_string()]
    );
}

#[test]
fn truncate_for_log_appends_ellipsis_beyond_the_limit() {
    assert_eq!(truncate_for_log("abcdef", 3), "abc...");
    assert_eq!(truncate_for_log("abc", 3), "abc");
}
