//! 設定からレンダリング用の値を導く。
//!
//! 「どのディレクトリに音色があるか」と「そこから `cmrt_core::CoreConfig` をどう組むか」
//! だけを持つ。notepad / DAW / offline render / 各サーバーで共有する。

use std::path::{Path, PathBuf};

use crate::Config;

pub fn configured_patch_dirs(cfg: &Config) -> Vec<String> {
    cfg.patches_dirs
        .clone()
        .unwrap_or_default()
        .into_iter()
        .filter(|dir| !dir.trim().is_empty())
        .collect()
}

pub fn core_config_patch_root_dir(cfg: &Config) -> Option<String> {
    shared_patch_root_dir(&configured_patch_dirs(cfg))
}

/// アプリ設定からレンダリング用の `CoreConfig` を組み立てる。
/// notepad / DAW / offline render / server の各経路で共有する。
pub fn core_config_from_config(cfg: &Config) -> cmrt_core::CoreConfig {
    cmrt_core::CoreConfig {
        output_midi: cfg.output_midi.clone(),
        output_wav: cfg.output_wav.clone(),
        sample_rate: cfg.sample_rate,
        buffer_size: cfg.buffer_size,
        patch_path: None,
        patches_dir: core_config_patch_root_dir(cfg),
        random_patch: false,
    }
}

pub fn shared_patch_root_dir(dirs: &[String]) -> Option<String> {
    let mut dir_paths = dirs.iter().map(PathBuf::from);
    let mut common = dir_paths.next()?;
    for dir in dir_paths {
        while !Path::new(&dir).starts_with(&common) {
            if !common.pop() {
                return None;
            }
        }
    }
    if common.as_os_str().is_empty() {
        return None;
    }
    Some(common.to_string_lossy().into_owned())
}
