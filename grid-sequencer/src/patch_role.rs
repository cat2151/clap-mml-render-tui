//! 行の用途（[`PatchRole`]）と、この画面が持つ設定・voicing 判定との橋渡し。
//!
//! 「どのカテゴリから引くか」「poly を要求するか」といった Surge 固有の判断は
//! [`cmrt_surge_patches`] 側にあり、ここは context から材料を渡すだけ。

use cmrt_realtime_play::PatchVoicing;
use cmrt_rhythm::DrumRole;
use cmrt_surge_patches::{PatchRole, RoleFilter, VoicingLookup};

use crate::{GridSequencerContext, GridVoicingLookup};

/// この画面の voicing 判定を、patch catalog 側の trait へ橋渡しするアダプタ。
pub(crate) struct PolyLookup<'a>(&'a dyn GridVoicingLookup);

impl VoicingLookup for PolyLookup<'_> {
    fn is_poly(&self, patch: &str) -> bool {
        self.0.cached_voicing(patch) == Some(PatchVoicing::Poly)
    }
}

/// 用途ごとの候補判定の材料。
///
/// キーワードは小文字化して持つ必要があり、percussion では他3役ぶんの合併にもなる。
/// どちらも借用のままでは持てないので、この型が受け皿になる。
pub(crate) struct GridRoleFilter<'a> {
    role: PatchRole,
    categories: &'a [String],
    keywords: Vec<String>,
}

impl GridRoleFilter<'_> {
    pub(crate) fn filter(&self) -> RoleFilter<'_> {
        RoleFilter::with_keywords(self.role, self.categories, &self.keywords)
    }
}

/// drum の役割に対応する patch の用途。
pub(crate) fn drum_patch_role(role: DrumRole) -> PatchRole {
    match role {
        DrumRole::Kick => PatchRole::Kick,
        DrumRole::Snare => PatchRole::Snare,
        DrumRole::HiHat => PatchRole::HiHat,
        DrumRole::Percussion => PatchRole::Percussion,
    }
}

impl GridSequencerContext<'_> {
    /// 用途に対応する config の設定を当てて、候補判定を組み立てる。
    pub(crate) fn role_filter(&self, role: PatchRole) -> GridRoleFilter<'_> {
        let categories = match role {
            PatchRole::Bass => self.bass_patch_categories,
            PatchRole::Arpeggio => self.arpeggio_patch_categories,
            PatchRole::Kick | PatchRole::Snare | PatchRole::HiHat | PatchRole::Percussion => {
                self.drum_patch_categories
            }
            // Free は「chord 候補を避ける」判定なので、見るのは chord のカテゴリ。
            PatchRole::Chord | PatchRole::Free => self.chord_patch_categories,
        };
        GridRoleFilter {
            role,
            categories,
            keywords: self.role_keywords(role),
        }
    }

    /// 用途に対応する patch 名キーワード。打楽器以外は空。
    ///
    /// percussion は「他の3役に取られなかった残り全部」なので、渡すのは自分のぶんでは
    /// なく他3役ぶんの合併になる。
    ///
    /// 照合相手が小文字化した patch パスなので、ここで小文字へ揃える
    /// （config.toml に大文字で書かれていても当たるように）。
    fn role_keywords(&self, role: PatchRole) -> Vec<String> {
        let sources: &[&[String]] = match role {
            PatchRole::Kick => &[self.kick_patch_keywords],
            PatchRole::Snare => &[self.snare_patch_keywords],
            PatchRole::HiHat => &[self.hihat_patch_keywords],
            PatchRole::Percussion => &[
                self.kick_patch_keywords,
                self.snare_patch_keywords,
                self.hihat_patch_keywords,
            ],
            PatchRole::Chord | PatchRole::Bass | PatchRole::Arpeggio | PatchRole::Free => &[],
        };
        sources
            .iter()
            .flat_map(|keywords| keywords.iter())
            .map(|keyword| keyword.to_lowercase())
            .collect()
    }

    pub(crate) fn poly_lookup(&self) -> PolyLookup<'_> {
        PolyLookup(self.voicing)
    }
}
