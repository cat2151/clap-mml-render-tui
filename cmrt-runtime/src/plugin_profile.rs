//! `active_plugin` + `[plugins.*]` によるプラグイン切り替え。
//!
//! 解決規則そのもの（組み込みプロファイル・名前の表記ゆれ吸収・config 側での上書き）は
//! play server repo 側の [`cmrt_server_config`] が単一ソースとして持つ。ここはその結果を
//! [`Config`] のトップレベルフィールドへ焼き込む TUI 側の役割だけを担う。
//!
//! 焼き込んでおくと `cfg.plugin_path` / `cfg.patches_dirs` の読み手（app・各画面）が
//! プロファイルの存在を一切知らずに済む。
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
    apply_patch_roles(profile.patch_roles, cfg);
    Ok(())
}

/// 用途別 patch カテゴリのうち「書かれている項目」だけをトップレベルフィールドへ焼き込む。
///
/// `None` は「書かれていない」なのでトップレベルの既定値を残し、`[]` は「カテゴリで
/// 絞らない」という明示の指定なのでそのまま空で焼き込む。この区別が Dexed
/// （cartridge にカテゴリ階層が無い）の候補を全滅させないための要。
fn apply_patch_roles(roles: PatchRoleFilters, cfg: &mut Config) {
    let assignments: [(Option<Vec<String>>, &mut Vec<String>); 7] = [
        (
            roles.chord_patch_categories,
            &mut cfg.chord_patch_categories,
        ),
        (roles.bass_patch_categories, &mut cfg.bass_patch_categories),
        (
            roles.arpeggio_patch_categories,
            &mut cfg.arpeggio_patch_categories,
        ),
        (roles.drum_patch_categories, &mut cfg.drum_patch_categories),
        (roles.kick_patch_keywords, &mut cfg.kick_patch_keywords),
        (roles.snare_patch_keywords, &mut cfg.snare_patch_keywords),
        (roles.hihat_patch_keywords, &mut cfg.hihat_patch_keywords),
    ];
    for (from_profile, target) in assignments {
        if let Some(value) = from_profile {
            *target = value;
        }
    }
}

#[cfg(test)]
mod tests;
