//! 固定の既定プラグイン Surge XT と `[plugins.*]` の runtime adapter。
//!
//! 解決規則そのもの（組み込みプロファイル・名前の表記ゆれ吸収・config 側での上書き）は
//! play server repo 側の [`cmrt_server_config`] が単一ソースとして持つ。ここは固定の
//! `[plugins."Surge XT"]` の解決結果を [`Config`] の runtime field へ焼き込むだけである。
//!
//! 焼き込んでおくと `cfg.plugin_path` / `cfg.patches_dirs` の読み手（app・各画面）が
//! プロファイルの存在を一切知らずに済む。
//!
//! 焼き込み先を残すのは、`cfg.plugin_path` / `cfg.patches_dirs` を読む既存 caller を一斉変更せず
//! config 構文だけを profile 方式へ統一するため。

pub use cmrt_server_config::{builtin_plugin_profiles, PluginProfile};

use crate::Config;

/// 固定の Surge XT profile を既存の runtime field へ焼き込む。
///
/// これを [`Config::load`] の中で済ませておくことで、`cfg.plugin_path` /
/// `cfg.patches_dirs` の読み手（app・各画面）は一切変更せずに済む。
///
/// 組み込み値と `[plugins."Surge XT"]` の merge は
/// [`cmrt_server_config::resolve_primary_plugin_profile`] が単一ソース。
pub fn apply_primary_plugin_profile(cfg: &mut Config) -> anyhow::Result<()> {
    let profile = cmrt_server_config::resolve_primary_plugin_profile(&cfg.plugins)?;
    cfg.plugin_path = profile.plugin_path;
    cfg.plugin_id = profile.plugin_id;
    cfg.patches_dirs = profile.patches_dirs;
    Ok(())
}

#[cfg(test)]
mod tests;
