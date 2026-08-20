//! 用途別 patch 自動選択（grid sequencer の chord / bass / arpeggio / drum 行）の
//! 絞り込みを、プラグイン 1 つぶんへ解決した形。
//!
//! [`PatchRoleFilters`] は「書かれていない項目」を `None` で表す差分なので、そのままでは
//! 使えない。土台に差分を当てた結果がこの型。
//!
//! カタログに複数プラグインの音色が並ぶと、この 1 組では足りない。Surge のカテゴリを
//! Dexed の cartridge へ当てると候補が全滅する（cartridge にカテゴリ階層が無い）ので、
//! **プラグインごとに 1 組**持ち、patch 文字列で引き分ける。
//!
//! # 3 層で解く
//!
//! 上から順に「書かれている項目」を採る。
//!
//! 1. `[plugins.<名前>]` に書かれた項目
//! 2. config トップレベルに書かれた項目（レガシー綴り）。**既定プラグインにだけ**効く
//! 3. そのプラグインの組み込み既定（[`PatchRoles::builtin_for`]）
//!
//! **層 2 を全プラグインの土台にしてはいけない。** トップレベルの値は Surge XT の
//! カテゴリ名（`active_plugin` が無かった時代の唯一のプラグイン）なので、それを土台に
//! すると、プロファイルを持たない `[plugins.my_synth]` をカタログへ足したときに
//! Surge のカテゴリで絞られて候補が全滅する（`docs/adr/0007-patch-role-defaults-three-layers.md`）。
//! 層 2 を既定プラグイン限定にしておくと、既存 config は今までどおりの結果になり、
//! 新しく足したプラグインだけが層 3（そのプラグインの既定）へ落ちる。

use crate::{is_surge_xt_plugin, Config, PatchRoleFilters};

/// プラグイン 1 つぶんの、解決済みの用途別絞り込み。
///
/// カテゴリが空なら「カテゴリでは絞らない」、キーワードが空なら「キーワードでは絞らない」。
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PatchRoles {
    pub chord_patch_categories: Vec<String>,
    pub bass_patch_categories: Vec<String>,
    pub arpeggio_patch_categories: Vec<String>,
    pub drum_patch_categories: Vec<String>,
    pub kick_patch_keywords: Vec<String>,
    pub snare_patch_keywords: Vec<String>,
    pub hihat_patch_keywords: Vec<String>,
}

impl PatchRoles {
    /// このプラグインの組み込み既定（層 3）。
    ///
    /// カテゴリ階層を持つ音色置き場を使うプラグインでなければ「絞らない」が正解。
    /// カテゴリ名は音色置き場の体系ごとの知識なので、Surge XT のぶんは
    /// [`cmrt_patches::surge_xt`] が単一ソースとして持つ。
    ///
    /// **組み込みプロファイル（play server repo 側の `builtin_plugin_profiles`）へは
    /// 書けない。** そちらへ書くと PS がカテゴリ名を知ることになり、いったん消した
    /// 「PS → TUI」の依存が復活する（`docs/adr/0007-patch-role-defaults-three-layers.md`）。
    pub fn builtin_for(plugin_id: Option<&str>, plugin_path: &str) -> Self {
        if is_surge_xt_plugin(plugin_id, plugin_path) {
            Self::surge_xt()
        } else {
            Self::default()
        }
    }

    /// Surge XT の音色置き場（`patches_factory` / `patches_3rdparty`）向けの既定。
    fn surge_xt() -> Self {
        use cmrt_patches::surge_xt::{
            DEFAULT_ARPEGGIO_PATCH_CATEGORY_NAMES, DEFAULT_BASS_PATCH_CATEGORY_NAMES,
            DEFAULT_CHORD_PATCH_CATEGORY_NAMES, DEFAULT_DRUM_PATCH_CATEGORY_NAMES,
            DEFAULT_HIHAT_PATCH_KEYWORDS, DEFAULT_KICK_PATCH_KEYWORDS,
            DEFAULT_SNARE_PATCH_KEYWORDS,
        };
        Self {
            chord_patch_categories: to_owned(&DEFAULT_CHORD_PATCH_CATEGORY_NAMES),
            bass_patch_categories: to_owned(&DEFAULT_BASS_PATCH_CATEGORY_NAMES),
            arpeggio_patch_categories: to_owned(&DEFAULT_ARPEGGIO_PATCH_CATEGORY_NAMES),
            drum_patch_categories: to_owned(&DEFAULT_DRUM_PATCH_CATEGORY_NAMES),
            kick_patch_keywords: to_owned(&DEFAULT_KICK_PATCH_KEYWORDS),
            snare_patch_keywords: to_owned(&DEFAULT_SNARE_PATCH_KEYWORDS),
            hihat_patch_keywords: to_owned(&DEFAULT_HIHAT_PATCH_KEYWORDS),
        }
    }

    /// プロファイルの「書かれている項目」を、そのプラグインの組み込み既定へ当てる。
    ///
    /// `None` は「書かれていない」なので `builtin` の値を使い、`[]` は「絞らない」という
    /// 明示の指定なのでそのまま空になる。この区別が Dexed（cartridge にカテゴリ階層が
    /// 無い）の候補を全滅させないための要。
    pub fn resolve(from_profile: &PatchRoleFilters, builtin: &PatchRoles) -> Self {
        let pick = |written: &Option<Vec<String>>, builtin: &Vec<String>| {
            written.clone().unwrap_or_else(|| builtin.clone())
        };
        Self {
            chord_patch_categories: pick(
                &from_profile.chord_patch_categories,
                &builtin.chord_patch_categories,
            ),
            bass_patch_categories: pick(
                &from_profile.bass_patch_categories,
                &builtin.bass_patch_categories,
            ),
            arpeggio_patch_categories: pick(
                &from_profile.arpeggio_patch_categories,
                &builtin.arpeggio_patch_categories,
            ),
            drum_patch_categories: pick(
                &from_profile.drum_patch_categories,
                &builtin.drum_patch_categories,
            ),
            kick_patch_keywords: pick(
                &from_profile.kick_patch_keywords,
                &builtin.kick_patch_keywords,
            ),
            snare_patch_keywords: pick(
                &from_profile.snare_patch_keywords,
                &builtin.snare_patch_keywords,
            ),
            hihat_patch_keywords: pick(
                &from_profile.hihat_patch_keywords,
                &builtin.hihat_patch_keywords,
            ),
        }
    }

    /// 既定プラグイン（音色無指定の行が鳴るもの）ぶんの解決。
    ///
    /// プロファイルとその組み込み既定の間に、config トップレベルの値を挟む。
    /// **トップレベルが効くのはここだけ**（module doc の層 2）。
    pub fn resolve_for_default_plugin(cfg: &Config, from_profile: &PatchRoleFilters) -> Self {
        Self::resolve(
            &layered_patch_role_filters(from_profile, &cfg.top_level_patch_roles),
            &Self::builtin_for(cfg.plugin_id.as_deref(), &cfg.plugin_path),
        )
    }
}

/// `over` の「書かれている項目」を `under` に重ねる。どちらも差分のまま。
///
/// 層を 1 つずつ潰していくのではなく差分を畳んでから解決するのは、
/// 「`[]` は明示の指定」という区別を最後まで壊さないため。
pub fn layered_patch_role_filters(
    over: &PatchRoleFilters,
    under: &PatchRoleFilters,
) -> PatchRoleFilters {
    let pick = |over: &Option<Vec<String>>, under: &Option<Vec<String>>| {
        over.clone().or_else(|| under.clone())
    };
    PatchRoleFilters {
        chord_patch_categories: pick(&over.chord_patch_categories, &under.chord_patch_categories),
        bass_patch_categories: pick(&over.bass_patch_categories, &under.bass_patch_categories),
        arpeggio_patch_categories: pick(
            &over.arpeggio_patch_categories,
            &under.arpeggio_patch_categories,
        ),
        drum_patch_categories: pick(&over.drum_patch_categories, &under.drum_patch_categories),
        kick_patch_keywords: pick(&over.kick_patch_keywords, &under.kick_patch_keywords),
        snare_patch_keywords: pick(&over.snare_patch_keywords, &under.snare_patch_keywords),
        hihat_patch_keywords: pick(&over.hihat_patch_keywords, &under.hihat_patch_keywords),
    }
}

fn to_owned(names: &[&str]) -> Vec<String> {
    names.iter().map(|name| (*name).to_string()).collect()
}

#[cfg(test)]
mod tests;
