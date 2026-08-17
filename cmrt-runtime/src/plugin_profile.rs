//! `active_plugin` + `[plugins.*]` によるプラグイン切り替え。
//!
//! 解決した値は [`Config`] のトップレベルフィールドへ焼き込む。こうしておくと
//! `cfg.plugin_path` / `cfg.patches_dirs` の読み手（app・各サーバー）が
//! プロファイルの存在を一切知らずに済む。

use serde::Deserialize;

use crate::Config;

/// `[plugins.<名前>]` 1 つ分のプラグイン設定。
///
/// Surge XT と Dexed の両方を config に残したまま、[`Config::active_plugin`] 1 行で
/// 行き来できるようにするためのもの。
#[derive(Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct PluginProfile {
    pub plugin_path: String,
    /// 期待する CLAP plugin ID。診断用の任意項目。
    #[serde(default)]
    pub plugin_id: Option<String>,
    /// このプラグインの音色置き場。書かなければ「音色置き場は無い」を意味する。
    #[serde(default)]
    pub patches_dirs: Option<Vec<String>>,
}

/// `active_plugin` が指すプロファイルの値をトップレベルフィールドへ焼き込む。
///
/// これを [`Config::load`] の中で済ませておくことで、`cfg.plugin_path` /
/// `cfg.patches_dirs` の読み手（app・各サーバー）は一切変更せずに済む。
///
/// - `active_plugin` が未指定なら何もしない（既存 config の完全な後方互換）。
/// - `active_plugin` が指す `[plugins.*]` が無ければ設定エラー。
/// - トップレベルの `plugin_path` と併記されていてもエラーにせず、プロファイルを優先する。
///   移行の途中で必ず引っかかるような conflict error にはしない。
/// - `patches_dirs` はプロファイルの値で**置き換える**。書かれていなければ「音色置き場は
///   無い」を意味し、トップレベル（Surge 用）の値は流用しない。別プラグインに Surge の
///   `.fxp` 一覧を見せないため。
pub fn apply_active_plugin_profile(cfg: &mut Config) -> anyhow::Result<()> {
    let Some(name) = cfg.active_plugin.as_deref().map(str::trim) else {
        return Ok(());
    };
    if name.is_empty() {
        anyhow::bail!("active_plugin に空の名前は指定できません");
    }
    let Some(profile) = cfg.plugins.get(name).cloned() else {
        let available = if cfg.plugins.is_empty() {
            "(1 つも定義されていません)".to_string()
        } else {
            cfg.plugins
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>()
                .join(", ")
        };
        anyhow::bail!(
            "active_plugin = '{name}' に対応する [plugins.{name}] がありません。\
             定義済みのプロファイル: {available}"
        );
    };
    if profile.plugin_path.trim().is_empty() {
        anyhow::bail!("[plugins.{name}] の plugin_path が空です");
    }
    if !cfg.plugin_path.trim().is_empty() {
        eprintln!(
            "config.toml: active_plugin = '{name}' のプロファイルを使います\
             （トップレベルの plugin_path / patches_dirs は無視します）"
        );
    }
    cfg.plugin_path = profile.plugin_path;
    cfg.plugin_id = profile.plugin_id;
    cfg.patches_dirs = profile.patches_dirs;
    Ok(())
}

#[cfg(test)]
mod tests;
