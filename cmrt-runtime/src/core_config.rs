//! 設定から「どのディレクトリに音色があるか」を導く。
//!
//! 畳み込みの規則そのものは play server repo 側の [`cmrt_server_config`] が単一ソース。
//! ここは [`Config`] を受け取る形へ合わせるだけの薄い層で、呼び出し側（app・tui-core・
//! daw など）のシグネチャを変えないために残している。
//!
//! ここから `CoreConfig` を組む処理は core-lib 側（`cmrt_core::core_config_from_config`）に
//! 置いてある。この crate を config 専用の葉 crate に保つため。

pub use cmrt_server_config::shared_patch_root_dir;

use crate::Config;

pub fn configured_patch_dirs(cfg: &Config) -> Vec<String> {
    cmrt_server_config::configured_patch_dirs(cfg.patches_dirs.as_deref())
}

pub fn core_config_patch_root_dir(cfg: &Config) -> Option<String> {
    cmrt_server_config::patch_root_dir(cfg.patches_dirs.as_deref())
}
