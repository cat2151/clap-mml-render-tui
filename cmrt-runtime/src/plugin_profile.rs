//! 固定の既定プラグイン Surge XT と `[plugins.*]` の runtime adapter。
//!
//! 解決規則（組み込みプロファイル・名前の表記ゆれ吸収・config 側での上書き）は
//! play server repo 側の [`cmrt_server_config`] が単一ソース。ここは `[plugins."Surge XT"]` の
//! 解決結果を [`Config`] の `plugin_path` / `patches_dirs` へ焼き込むだけで、焼き込むのは
//! それらの読み手（app・各画面）にプロファイルの存在を知らせないため。

pub use cmrt_server_config::{builtin_plugin_profiles, PluginProfile};

use crate::Config;

/// 固定の Surge XT profile を runtime field へ焼き込む。[`Config::load`] の中で呼ぶ。
pub fn apply_primary_plugin_profile(cfg: &mut Config) -> anyhow::Result<()> {
    let profile = cmrt_server_config::resolve_primary_plugin_profile(&cfg.plugins)?;
    cfg.plugin_path = profile.plugin_path;
    cfg.plugin_id = profile.plugin_id;
    cfg.patches_dirs = profile.patches_dirs;
    Ok(())
}

#[cfg(test)]
mod tests;
