//! patch の display 文字列を解釈・分類・抽選する crate。
//!
//! 並べ替え・名前の突き合わせ・用途別の抽選（[`grouping`] / [`naming`] /
//! [`selection`]）はプラグインに依らない。プラグイン固有なのは「patch 文字列の
//! どこがカテゴリか」「用途ごとの既定カテゴリ名は何か」だけで、それを
//! **プラグインごとの module へ分けて持つ**:
//!
//! - [`surge_xt`] — `patches_factory/<category>/` と `patches_3rdparty/<vendor>/<category>/`
//! - [`cartridge`] — Dexed の `<cartridge>.syx/<voice>`
//!
//! どちらで読むかを選ぶ中立の入口が [`layout`]（[`layout::PatchLayout`]）。
//! プラグインを足すときはこの 2 つに並べて 3 つめを足し、`PatchLayout` へ分岐を
//! 1 本増やす。
//!
//! 扱うのは「列挙済みのパス列を解釈・分類・抽選する」ところまで。patch ファイルの
//! 列挙そのもの（`.fxp` / `.syx` の走査）は別 repo（`clap-mml-play-server`）の
//! 責務なのでここには持ち込まない。
//!
//! patch は全体を通して `(表示名, 小文字化した表示名)` のペアで受け渡す。
//! 小文字化した側をパス照合に、表示名側を voicing 判定と結果の返却に使う。

pub mod cartridge;
mod grouping;
pub mod layout;
mod naming;
mod selection;
pub mod surge_xt;

pub use grouping::{
    group_patch_pairs_by_category, sort_patch_pairs, PatchCategory, PatchSortOrder,
};
pub use layout::{patch_category, patch_matches_categories, PatchLayout};
pub use naming::{
    compare_normalized_patch_names_natural, compare_patch_names_natural,
    normalize_patch_lookup_key, resolve_display_patch_name,
};
pub use selection::{
    candidates_for_role, matches_role, pick_for_role, PatchRole, RoleFilter, RoleFilterLookup,
    VoicingLookup,
};
