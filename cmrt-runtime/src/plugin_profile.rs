//! `active_plugin` + `[plugins.*]` によるプラグイン切り替え。
//!
//! 解決した値は [`Config`] のトップレベルフィールドへ焼き込む。こうしておくと
//! `cfg.plugin_path` / `cfg.patches_dirs` の読み手（app・各サーバー）が
//! プロファイルの存在を一切知らずに済む。
//!
//! 既知のプラグインは [`builtin_plugin_profiles`] に組み込みで持っている。
//! 標準の場所へインストールしてあるなら `active_plugin = 'Dexed'` の 1 行だけでよく、
//! `[plugins.*]` を書く必要はない。

use std::collections::BTreeMap;

use serde::Deserialize;

use crate::{default_dexed_plugin_path, default_patches_dirs, default_plugin_path, Config};

/// `[plugins.<名前>]` 1 つ分のプラグイン設定。
///
/// Surge XT と Dexed の両方を config に残したまま、[`Config::active_plugin`] 1 行で
/// 行き来できるようにするためのもの。
///
/// 各項目は「書かなければ組み込みプロファイルの値を引き継ぐ」。`patches_dirs` を
/// 明示的に空にしたいときは `patches_dirs = []` と書く。
#[derive(Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct PluginProfile {
    #[serde(default)]
    pub plugin_path: String,
    /// 期待する CLAP plugin ID。診断用の任意項目。
    #[serde(default)]
    pub plugin_id: Option<String>,
    /// このプラグインの音色置き場。
    #[serde(default)]
    pub patches_dirs: Option<Vec<String>>,
}

impl PluginProfile {
    /// `self` を土台に `over` の「書かれている項目」だけを上書きする。
    fn overridden_by(self, over: PluginProfile) -> Self {
        Self {
            plugin_path: if over.plugin_path.trim().is_empty() {
                self.plugin_path
            } else {
                over.plugin_path
            },
            plugin_id: over.plugin_id.or(self.plugin_id),
            patches_dirs: over.patches_dirs.or(self.patches_dirs),
        }
    }
}

/// config に何も書かなくても使える組み込みプロファイル。
///
/// パスは OS ごとの標準インストール先（`default_plugin_path()` などと同じ根拠）。
/// 別の場所に入れている場合だけ `[plugins.<名前>]` に `plugin_path` を書けばよく、
/// `plugin_id` や `patches_dirs` はここの値が引き継がれる。
pub fn builtin_plugin_profiles() -> BTreeMap<String, PluginProfile> {
    BTreeMap::from([
        (
            "Surge XT".to_string(),
            PluginProfile {
                plugin_path: default_plugin_path().to_string(),
                plugin_id: Some("org.surge-synth-team.surge-xt".to_string()),
                patches_dirs: Some(default_patches_dirs()),
            },
        ),
        (
            "Dexed".to_string(),
            PluginProfile {
                plugin_path: default_dexed_plugin_path().to_string(),
                plugin_id: Some("com.digital-suburban.dexed".to_string()),
                // Dexed の音色選択は未対応なので音色置き場は持たない。
                patches_dirs: None,
            },
        ),
    ])
}

/// 表記ゆれを吸収するための比較用キー。`Surge XT` / `surge_xt` / `SurgeXT` を同一視する。
fn normalized(name: &str) -> String {
    name.chars()
        .filter(|c| !c.is_whitespace() && *c != '_' && *c != '-')
        .flat_map(char::to_lowercase)
        .collect()
}

/// 完全一致を優先し、無ければ表記ゆれを吸収して探す。
fn lookup(profiles: &BTreeMap<String, PluginProfile>, name: &str) -> Option<PluginProfile> {
    if let Some(profile) = profiles.get(name) {
        return Some(profile.clone());
    }
    let key = normalized(name);
    profiles
        .iter()
        .find(|(candidate, _)| normalized(candidate) == key)
        .map(|(_, profile)| profile.clone())
}

fn unknown_active_plugin_error(name: &str, cfg: &Config) -> anyhow::Error {
    let builtin = builtin_plugin_profiles()
        .keys()
        .cloned()
        .collect::<Vec<_>>()
        .join(", ");
    let configured = if cfg.plugins.is_empty() {
        "(config に [plugins.*] は書かれていません)".to_string()
    } else {
        cfg.plugins
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(", ")
    };
    anyhow::anyhow!(
        "active_plugin = '{name}' に対応するプロファイルがありません。\
         組み込みで使える名前: {builtin} / config で定義済みの名前: {configured}"
    )
}

/// `active_plugin` が指すプロファイルの値をトップレベルフィールドへ焼き込む。
///
/// これを [`Config::load`] の中で済ませておくことで、`cfg.plugin_path` /
/// `cfg.patches_dirs` の読み手（app・各サーバー）は一切変更せずに済む。
///
/// - `active_plugin` が未指定なら何もしない（既存 config の完全な後方互換）。
/// - 名前は [`builtin_plugin_profiles`] と config の `[plugins.*]` の両方から探す。
///   同名なら config 側の「書かれている項目」が組み込みを上書きする。
/// - どちらにも無ければ設定エラー。使える名前を両方とも並べて示す。
/// - トップレベルの `plugin_path` と併記されていてもエラーにせず、プロファイルを優先する。
///   移行の途中で必ず引っかかるような conflict error にはしない。
pub fn apply_active_plugin_profile(cfg: &mut Config) -> anyhow::Result<()> {
    let Some(name) = cfg.active_plugin.as_deref().map(str::trim) else {
        return Ok(());
    };
    if name.is_empty() {
        anyhow::bail!("active_plugin に空の名前は指定できません");
    }
    let configured = lookup(&cfg.plugins, name);
    let builtin = lookup(&builtin_plugin_profiles(), name);
    let profile = match (builtin, configured) {
        (Some(builtin), Some(configured)) => builtin.overridden_by(configured),
        (Some(builtin), None) => builtin,
        (None, Some(configured)) => configured,
        (None, None) => return Err(unknown_active_plugin_error(name, cfg)),
    };
    if profile.plugin_path.trim().is_empty() {
        anyhow::bail!(
            "active_plugin = '{name}' の plugin_path が空です。\
             [plugins.{name}] に plugin_path を書いてください"
        );
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
