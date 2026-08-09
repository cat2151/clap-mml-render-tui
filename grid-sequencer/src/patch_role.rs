//! 行の用途（[`PatchRole`]）と、この画面が持つ設定・voicing 判定との橋渡し。
//!
//! 「どのカテゴリから引くか」「poly を要求するか」といった Surge 固有の判断は
//! [`cmrt_surge_patches`] 側にあり、ここは context から材料を渡すだけ。

use cmrt_realtime_play::PatchVoicing;
use cmrt_surge_patches::{PatchRole, RoleFilter, VoicingLookup};

use crate::{GridSequencerContext, GridVoicingLookup};

/// この画面の voicing 判定を、patch catalog 側の trait へ橋渡しするアダプタ。
pub(crate) struct PolyLookup<'a>(&'a dyn GridVoicingLookup);

impl VoicingLookup for PolyLookup<'_> {
    fn is_poly(&self, patch: &str) -> bool {
        self.0.cached_voicing(patch) == Some(PatchVoicing::Poly)
    }
}

impl GridSequencerContext<'_> {
    /// 用途に対応する config のカテゴリ設定を当てて、候補判定を組み立てる。
    pub(crate) fn role_filter(&self, role: PatchRole) -> RoleFilter<'_> {
        let categories = match role {
            PatchRole::Bass => self.bass_patch_categories,
            PatchRole::Arpeggio => self.arpeggio_patch_categories,
            // Free は「chord 候補を避ける」判定なので、見るのは chord のカテゴリ。
            PatchRole::Chord | PatchRole::Free => self.chord_patch_categories,
        };
        RoleFilter::new(role, categories)
    }

    pub(crate) fn poly_lookup(&self) -> PolyLookup<'_> {
        PolyLookup(self.voicing)
    }
}
