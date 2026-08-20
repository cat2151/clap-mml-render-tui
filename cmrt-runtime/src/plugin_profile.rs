//! `active_plugin` + `[plugins.*]` によるプラグイン切り替え。
//!
//! 解決規則そのもの（組み込みプロファイル・名前の表記ゆれ吸収・config 側での上書き）は
//! play server repo 側の [`cmrt_server_config`] が単一ソースとして持つ。ここはその結果を
//! [`Config`] のトップレベルフィールドへ焼き込む TUI 側の役割だけを担う。
//!
//! 焼き込んでおくと `cfg.plugin_path` / `cfg.patches_dirs` の読み手（app・各画面）が
//! プロファイルの存在を一切知らずに済む。
//!
//! 用途別 patch カテゴリ（[`PatchRoleFilters`]）だけは焼き込まない。トップレベルの値は
//! 「プロファイルが書いていない項目の土台」で、カタログに複数プラグインが並ぶと
//! プラグインごとに別の解決結果が要るため。解決は [`crate::PatchRoles::resolve`]。
//!
//! 既知のプラグインは [`builtin_plugin_profiles`] に組み込みで持っている。
//! 標準の場所へインストールしてあるなら `active_plugin = 'Dexed'` の 1 行だけでよく、
//! `[plugins.*]` を書く必要はない。

pub use cmrt_server_config::{builtin_plugin_profiles, PatchRoleFilters, PluginProfile};

use crate::Config;

/// `active_plugin` が指すプロファイルの値をトップレベルフィールドへ焼き込む。
///
/// これを [`Config::load`] の中で済ませておくことで、`cfg.plugin_path` /
/// `cfg.patches_dirs` の読み手（app・各画面）は一切変更せずに済む。
///
/// 解決の規則（未指定なら何もしない・組み込みと config の両方から探す・見つからなければ
/// 使える名前を並べたエラー）は [`cmrt_server_config::resolve_active_plugin_profile`] 側にある。
pub fn apply_active_plugin_profile(cfg: &mut Config) -> anyhow::Result<()> {
    let Some(profile) = cmrt_server_config::resolve_active_plugin_profile(
        cfg.active_plugin.as_deref(),
        &cfg.plugins,
        &cfg.plugin_path,
    )?
    else {
        return Ok(());
    };
    cfg.plugin_path = profile.plugin_path;
    cfg.plugin_id = profile.plugin_id;
    cfg.patches_dirs = profile.patches_dirs;
    // 用途別カテゴリだけは焼き込まず、差分のまま持つ。トップレベルの値は
    // 「プロファイルが書いていない項目の土台」であり、カタログに複数プラグインが
    // 並ぶとプラグインごとに別の解決結果が要るため、土台を潰してはいけない。
    cfg.active_patch_roles = profile.patch_roles;
    Ok(())
}

#[cfg(test)]
mod tests;
