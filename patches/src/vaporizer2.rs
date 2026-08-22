//! Vaporizer2（VAST Dynamics）の patch 体系。
//!
//! display 文字列は音色置き場からの相対パスで、末尾が `.vvp`。1 ファイル = 1 音色
//! （`.syx` のような 32 program 展開は無い）で、出荷プリセットは**フラットな 1 階層**。
//! カテゴリは**ファイル名先頭 2 文字のコード**（`AR Accent Arp.vvp` の `AR`）で、
//! 展開名の表は [`layout`] が持つ。**このコード表を知っているのはこの module だけ。**
//!
//! 用途ごとの既定カテゴリ名（[`defaults`]）も Vaporizer2 の展開名なのでここに置く。
//! 中立の入口は [`crate::layout`] で、patch 文字列の形からここか [`crate::surge_xt`] か
//! [`crate::cartridge`] かを選ぶ。
//!
//! カテゴリを XML の `PatchCategory` 属性ではなくファイル名から取るのは、
//! **460 ファイル 681MB（最大 1 ファイル 17MB）を開かずに済ませるため**。
//! 実プリセット 460 件で属性とファイル名先頭 2 文字が完全に一致することを確かめてある。

mod defaults;
mod layout;

pub use defaults::{
    DEFAULT_ARPEGGIO_PATCH_CATEGORY_NAMES, DEFAULT_BASS_PATCH_CATEGORY_NAMES,
    DEFAULT_CHORD_PATCH_CATEGORY_NAMES, DEFAULT_DRUM_PATCH_CATEGORY_NAMES,
    DEFAULT_HIHAT_PATCH_KEYWORDS, DEFAULT_KICK_PATCH_KEYWORDS, DEFAULT_SNARE_PATCH_KEYWORDS,
};
pub use layout::{category_name_for_code, PATCH_CATEGORY_CODES};
pub(crate) use layout::{category_sort_parts, has_vvp_extension, path_sort_parts};

#[cfg(test)]
mod tests;
