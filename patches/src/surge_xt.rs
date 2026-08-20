//! Surge XT の patch 体系。
//!
//! display 文字列は `patches_factory/<category>/<patch>.fxp` と
//! `patches_3rdparty/<vendor>/<category>/<patch>.fxp`（vendor が無い形もある）の
//! 2 本立てで、先頭の prefix が供給元、その次（3rdparty は vendor を挟んだ次）が
//! カテゴリになる。**このディレクトリ構造を知っているのはこの module だけ。**
//!
//! 用途ごとの既定カテゴリ名（[`defaults`]）も Surge のカテゴリ名なのでここに置く。
//! 中立の入口は [`crate::layout`] で、patch 文字列の形からここか
//! [`crate::cartridge`] かを選ぶ。

mod defaults;
mod layout;

pub use defaults::{
    DEFAULT_ARPEGGIO_PATCH_CATEGORY_NAMES, DEFAULT_BASS_PATCH_CATEGORY_NAMES,
    DEFAULT_CHORD_PATCH_CATEGORY_NAMES, DEFAULT_DRUM_PATCH_CATEGORY_NAMES,
    DEFAULT_HIHAT_PATCH_KEYWORDS, DEFAULT_KICK_PATCH_KEYWORDS, DEFAULT_SNARE_PATCH_KEYWORDS,
};
pub use layout::PATCH_DIR_PREFIXES;
pub(crate) use layout::{category_sort_parts, has_known_prefix, path_sort_parts};

#[cfg(test)]
mod tests;
