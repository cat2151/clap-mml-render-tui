//! カートリッジ形式（Dexed の DX7 `.syx`）の patch 体系。
//!
//! display 文字列は `<cartridge>.syx/<voice 名>` の 1 階層きり。カートリッジ
//! ファイルそのものがカテゴリの役をする。供給元の区別が無いのでソート優先度は常に 0。
//!
//! 中立の入口は [`crate::layout`]。

/// カートリッジ形式には factory / 3rdparty の別が無い。
const SORT_PRIORITY: u8 = 0;

/// カテゴリ順ソート用に `(カテゴリ, 供給元の優先度, vendor, 残りのパス)` へ分解する。
/// vendor は常に空。
pub(crate) fn category_sort_parts(path: &str) -> (&str, u8, &str, &str) {
    let (category, rest) = crate::layout::split_first_path_segment(path);
    (category, SORT_PRIORITY, "", rest)
}

/// パス順ソート用に `(供給元の優先度, パス)` へ分解する。剥がす prefix が無い。
pub(crate) fn path_sort_parts(path: &str) -> (u8, &str) {
    (SORT_PRIORITY, path)
}

#[cfg(test)]
mod tests;
