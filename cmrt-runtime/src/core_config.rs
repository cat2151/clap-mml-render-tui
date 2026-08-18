//! 設定から「どのディレクトリに音色があるか」を導く。
//!
//! ここから `CoreConfig` を組む処理は core-lib 側（`cmrt_core::core_config_from_config`）に
//! 置いてある。この crate を config 専用の葉 crate に保ち、play server repo から参照しても
//! TUI の core-lib を巻き込まないようにするため。

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
