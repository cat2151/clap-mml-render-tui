//! 抽象化済みpatch metadataを分類・抽選するcrate。
//!
//! plugin固有のpath解釈はplay-server共有coreが行い、このcrateは返されたcategory・
//! sort metadataと、plugin非依存の検索・用途別抽選だけを扱う。
//!
//! patch は全体を通して `(表示名, 小文字化した表示名)` のペアで受け渡す。
//! 小文字化した側をパス照合に、表示名側を voicing 判定と結果の返却に使う。

mod grouping;
mod layout;
mod naming;
mod roles;

pub use grouping::{
    group_patch_pairs_by_category, sort_patch_pairs, PatchCategory, PatchSortOrder,
};
pub use layout::{patch_category, patch_matches_categories};
pub use naming::{
    compare_normalized_patch_names_natural, compare_patch_names_natural,
    normalize_patch_lookup_key, resolve_display_patch_name,
};
pub use roles::{
    builtin_role_presets, normalize_user_role_presets, DrumPatchRole, PatchRole, PatchRoleIndex,
    PatchRoleInput, PatchRolePreset,
};
