pub(super) use super::*;
pub(super) use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
pub(super) use std::sync::atomic::{AtomicUsize, Ordering};
pub(super) use tui_textarea::{CursorMove, TextArea};

mod filter_cache;
mod insert_mode;
mod normal_mode;
mod notepad_history;
mod patch_phrase;
mod patch_select;
mod session;

static NEXT_TEST_ID: AtomicUsize = AtomicUsize::new(0);

fn make_patches(items: &[&str]) -> Vec<(String, String)> {
    items
        .iter()
        .map(|&s| (s.to_string(), s.to_lowercase()))
        .collect()
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
        patches_dirs: Some(vec!["/tmp/patches".to_string()]),
        daw_tracks: 9,
        daw_measures: 8,
    }
}
