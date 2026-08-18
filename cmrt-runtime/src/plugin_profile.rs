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

use crate::{
    default_dexed_cartridge_dirs, default_dexed_plugin_path, default_patches_dirs,
    default_plugin_path, Config, DEXED_PLUGIN_ID, SURGE_XT_PLUGIN_ID,
};

/// `[plugins.<名前>]` 1 つ分のプラグイン設定。
///
/// Surge XT と Dexed の両方を config に残したまま、[`Config::active_plugin`] 1 行で
/// 行き来できるようにするためのもの。
///
/// 各項目は「書かなければ組み込みプロファイルの値を引き継ぐ」。`patches_dirs` を
/// 明示的に空にしたいときは `patches_dirs = []` と書く。
///
/// [`PatchRoleFilters`] の項目はトップレベルと同じキー名で、`[plugins.*]` の中へ
/// そのまま書ける。
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
    /// 用途別の patch 自動選択の絞り込み。トップレベルと同じキー名で書ける。
    #[serde(flatten)]
    pub patch_roles: PatchRoleFilters,
}

/// 用途別 patch 自動選択（grid sequencer の chord / bass / arpeggio / drum 行）の
/// 絞り込み設定のうち、プラグインごとに正解が違うもの。
///
/// トップレベルの既定値は Surge のカテゴリ名なので、cartridge を音色置き場にする
/// Dexed では 1 つも当たらない。プラグインごとの正解をここへ持たせる。
///
/// 各項目は `None` が「書かれていない」で、そのときトップレベルの値をそのまま使う。
/// `[]` は「カテゴリで絞らない」という**明示の指定**なので区別が要る。
#[derive(Deserialize, Debug, Clone, Default, PartialEq, Eq)]
pub struct PatchRoleFilters {
    #[serde(default)]
    pub chord_patch_categories: Option<Vec<String>>,
    #[serde(default)]
    pub bass_patch_categories: Option<Vec<String>>,
    #[serde(default)]
    pub arpeggio_patch_categories: Option<Vec<String>>,
    #[serde(default)]
    pub drum_patch_categories: Option<Vec<String>>,
    #[serde(default)]
    pub kick_patch_keywords: Option<Vec<String>>,
    #[serde(default)]
    pub snare_patch_keywords: Option<Vec<String>>,
    #[serde(default)]
    pub hihat_patch_keywords: Option<Vec<String>>,
}

impl PatchRoleFilters {
    /// どの項目でも絞らない設定。カテゴリ階層を持たない音色置き場のプラグイン用。
    ///
    /// cartridge のディレクトリ名（`SynprezFM` など）は用途と無関係なので、
    /// カテゴリで絞る前提そのものが成り立たない。絞らずに全 program を候補にする。
    pub fn unfiltered() -> Self {
        Self {
            chord_patch_categories: Some(Vec::new()),
            bass_patch_categories: Some(Vec::new()),
            arpeggio_patch_categories: Some(Vec::new()),
            drum_patch_categories: Some(Vec::new()),
            kick_patch_keywords: Some(Vec::new()),
            snare_patch_keywords: Some(Vec::new()),
            hihat_patch_keywords: Some(Vec::new()),
        }
    }

    /// `self` を土台に `over` の「書かれている項目」だけを上書きする。
    fn overridden_by(self, over: Self) -> Self {
        Self {
            chord_patch_categories: over.chord_patch_categories.or(self.chord_patch_categories),
            bass_patch_categories: over.bass_patch_categories.or(self.bass_patch_categories),
            arpeggio_patch_categories: over
                .arpeggio_patch_categories
                .or(self.arpeggio_patch_categories),
            drum_patch_categories: over.drum_patch_categories.or(self.drum_patch_categories),
            kick_patch_keywords: over.kick_patch_keywords.or(self.kick_patch_keywords),
            snare_patch_keywords: over.snare_patch_keywords.or(self.snare_patch_keywords),
            hihat_patch_keywords: over.hihat_patch_keywords.or(self.hihat_patch_keywords),
        }
    }

    /// 書かれている項目だけをトップレベルフィールドへ焼き込む。
    fn apply_to(self, cfg: &mut Config) {
        let assignments: [(Option<Vec<String>>, &mut Vec<String>); 7] = [
            (self.chord_patch_categories, &mut cfg.chord_patch_categories),
            (self.bass_patch_categories, &mut cfg.bass_patch_categories),
            (
                self.arpeggio_patch_categories,
                &mut cfg.arpeggio_patch_categories,
            ),
            (self.drum_patch_categories, &mut cfg.drum_patch_categories),
            (self.kick_patch_keywords, &mut cfg.kick_patch_keywords),
            (self.snare_patch_keywords, &mut cfg.snare_patch_keywords),
            (self.hihat_patch_keywords, &mut cfg.hihat_patch_keywords),
        ];
        for (from_profile, target) in assignments {
            if let Some(value) = from_profile {
                *target = value;
            }
        }
    }
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
            patch_roles: self.patch_roles.overridden_by(over.patch_roles),
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
                plugin_id: Some(SURGE_XT_PLUGIN_ID.to_string()),
                patches_dirs: Some(default_patches_dirs()),
                // カテゴリ設定のトップレベル既定値が Surge のものなので、書く必要が無い。
                patch_roles: PatchRoleFilters::default(),
            },
        ),
        (
            "Dexed".to_string(),
            PluginProfile {
                plugin_path: default_dexed_plugin_path().to_string(),
                plugin_id: Some(DEXED_PLUGIN_ID.to_string()),
                patches_dirs: Some(default_dexed_cartridge_dirs()),
                patch_roles: PatchRoleFilters::unfiltered(),
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
    profile.patch_roles.apply_to(cfg);
    Ok(())
}

#[cfg(test)]
mod tests;
