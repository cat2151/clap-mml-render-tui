use std::path::PathBuf;

pub use cmrt_core::core_config_from_config;
pub use cmrt_runtime::{
    configured_patch_dirs, core_config_patch_root_dir, default_loop_categories,
    default_patches_dirs, default_plugin_path, serialize_patches_dirs_line, shared_patch_root_dir,
    Config, OfflineRenderBackend, RealtimeAudioBackend, DEFAULT_CHORD_PROGRESSION_SOURCE,
    DEFAULT_OFFLINE_RENDER_SERVER_PORT, DEFAULT_OFFLINE_RENDER_SERVER_WORKERS,
    DEFAULT_OFFLINE_RENDER_WORKERS, DEFAULT_REALTIME_PLAY_SERVER_PORT,
    DEFAULT_VOICING_OVERRIDE_SOURCE, DEFAULT_VOICING_SHARED_SOURCE,
};

pub fn load() -> anyhow::Result<Config> {
    Config::load_with_default_content(default_config_content())
}

pub fn default_config_content() -> String {
    cmrt_runtime::default_config_content_with_app_settings(
        &crate::config_editor::default_config_editor_block(),
    )
}

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

pub fn scan_loops_log_file_path() -> Option<PathBuf> {
    config_app_dir().map(|d| d.join("log").join("scan-loops.log"))
}

#[cfg(test)]
mod tests;
