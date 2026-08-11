//! Surge XT の patch 体系を知る唯一の crate。
//!
//! 「`patches_factory/<category>/` と `patches_3rdparty/<vendor>/<category>/` という
//! ディレクトリ構造」「用途ごとの既定カテゴリ名」といった Surge 固有の知識を、
//! 画面 crate から追い出してここへ閉じ込める。Surge 以外の CLAP を足すときの
//! 差し替え点が1箇所で済むようにするのが目的。
//!
//! 扱うのは「列挙済みのパス列を解釈・分類・抽選する」ところまで。patch ファイルの
//! 列挙そのもの（`.fxp` の走査）は別 repo（`clap-mml-play-server`）の責務なので
//! ここには持ち込まない。
//!
//! patch は全体を通して `(表示名, 小文字化した表示名)` のペアで受け渡す。
//! 小文字化した側をパス照合に、表示名側を voicing 判定と結果の返却に使う。

mod defaults;
mod grouping;
mod layout;
mod naming;
mod selection;

pub use defaults::{
    DEFAULT_ARPEGGIO_PATCH_CATEGORY_NAMES, DEFAULT_BASS_PATCH_CATEGORY_NAMES,
    DEFAULT_CHORD_PATCH_CATEGORY_NAMES, DEFAULT_DRUM_PATCH_CATEGORY_NAMES,
    DEFAULT_HIHAT_PATCH_KEYWORDS, DEFAULT_KICK_PATCH_KEYWORDS, DEFAULT_SNARE_PATCH_KEYWORDS,
};
pub use grouping::{
    group_patch_pairs_by_category, sort_patch_pairs, PatchCategory, PatchSortOrder,
};
pub use layout::{patch_category, patch_matches_categories};
pub use naming::{
    compare_normalized_patch_names_natural, compare_patch_names_natural,
    normalize_patch_lookup_key, resolve_display_patch_name,
};
pub use selection::{matches_role, pick_for_role, PatchRole, RoleFilter, VoicingLookup};
