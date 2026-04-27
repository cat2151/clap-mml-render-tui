use std::path::PathBuf;

use cmrt_core::CoreConfig;

pub use cmrt_runtime::{
    configured_patch_dirs, core_config_patch_root_dir, default_config_content,
    default_patches_dirs, default_plugin_path, serialize_patches_dirs_line, shared_patch_root_dir,
    Config, OfflineRenderBackend, RealtimeAudioBackend, DEFAULT_OFFLINE_RENDER_SERVER_PORT,
    DEFAULT_OFFLINE_RENDER_SERVER_WORKERS, DEFAULT_OFFLINE_RENDER_WORKERS,
    DEFAULT_REALTIME_PLAY_SERVER_PORT,
};

pub fn config_app_dir() -> Option<PathBuf> {
    #[cfg(test)]
    if let Some(app_dir) = crate::test_utils::test_app_dir_for_current_thread_or_default() {
        return Some(app_dir);
    }

    cmrt_runtime::config_app_dir()
}

pub fn config_file_path() -> Option<PathBuf> {
    config_app_dir().map(|d| d.join("config.toml"))
}

pub fn log_file_path() -> Option<PathBuf> {
    config_app_dir().map(|d| d.join("log").join("log.txt"))
}

pub fn native_probe_log_file_path() -> Option<PathBuf> {
    config_app_dir().map(|d| d.join("log").join("native_probe.log"))
}

pub fn core_config_from_config(cfg: &Config) -> CoreConfig {
    CoreConfig {
        output_midi: cfg.output_midi.clone(),
        output_wav: cfg.output_wav.clone(),
        sample_rate: cfg.sample_rate,
        buffer_size: cfg.buffer_size,
        patch_path: None,
        patches_dir: core_config_patch_root_dir(cfg),
        random_patch: false,
    }
}

#[cfg(test)]
#[path = "tests/config.rs"]
mod tests;
