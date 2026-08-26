pub(super) use super::*;
pub(super) use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
pub(super) use std::sync::atomic::{AtomicUsize, Ordering};

mod floe_screens;
mod keyboard_mml;
mod mml_overlay;
mod normal_mode;
mod screen_switch;
mod session;
mod session_bpm;
mod sforzando_screens;
mod vaporizer2_screens;

pub(super) static NEXT_TEST_ID: AtomicUsize = AtomicUsize::new(0);

fn make_patches(items: &[&str]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|&s| (s.to_string(), s.to_lowercase()))
        .collect()
}

pub(super) fn test_config() -> crate::config::Config {
    crate::config::Config {
        plugin_path: "/tmp/Surge XT.clap".to_string(),
        input_midi: "input.mid".to_string(),
        output_midi: "output.mid".to_string(),
        output_wav: "output.wav".to_string(),
        sample_rate: 44_100.0,
        buffer_size: 512,
        patches_dirs: Some(vec!["/tmp/patches".to_string()]),
        loop_dirs: Vec::new(),
        loop_categories: crate::config::default_loop_categories(),
        offline_render_workers: crate::config::DEFAULT_OFFLINE_RENDER_WORKERS,
        offline_render_server_workers: crate::config::DEFAULT_OFFLINE_RENDER_SERVER_WORKERS,
        offline_render_backend: crate::config::OfflineRenderBackend::InProcess,
        offline_render_server_port: crate::config::DEFAULT_OFFLINE_RENDER_SERVER_PORT,
        offline_render_server_command: String::new(),
        realtime_audio_backend: crate::config::RealtimeAudioBackend::InProcess,
        realtime_play_server_port: crate::config::DEFAULT_REALTIME_PLAY_SERVER_PORT,
        realtime_play_server_command: String::new(),
        realtime_play_server_prewarm: false,
        autoplay_on_startup: true,
        voicing_shared_source: String::new(),
        voicing_override_source: String::new(),
        chord_progression_source: String::new(),
        ..Default::default()
    }
}
