//! `patches_factory/` / `patches_3rdparty/` の prefix 解析。

use crate::layout::split_first_path_segment;

/// patch パスの先頭に来るディレクトリ名。factory を先に置き、ソート優先度と対応させる。
pub const PATCH_DIR_PREFIXES: [&str; 2] = ["patches_factory", "patches_3rdparty"];
const FACTORY_SORT_PRIORITY: u8 = 0;
const THIRD_PARTY_SORT_PRIORITY: u8 = 1;

/// この patch 文字列が Surge の prefix を持つか。
///
/// 持たないものは [`crate::cartridge`] の読み方へ回る。**prefix 抜きで保存された
/// Surge の名前もそちらへ落ちるが、どちらも先頭セグメントをカテゴリとして読むので
/// 結果は変わらない**（`crate::layout::PatchLayout` の doc も参照）。
pub(crate) fn has_known_prefix(path: &str) -> bool {
    PATCH_DIR_PREFIXES
        .iter()
        .any(|prefix| path.starts_with(&format!("{prefix}/")))
}

/// カテゴリ順ソート用に `(カテゴリ, 供給元の優先度, vendor, 残りのパス)` へ分解する。
pub(crate) fn category_sort_parts(path: &str) -> (&str, u8, &str, &str) {
    if let Some(rest) = path.strip_prefix("patches_factory/") {
        let (category, rest) = split_first_path_segment(rest);
        return (category, FACTORY_SORT_PRIORITY, "", rest);
    }

    let rest = path
        .strip_prefix("patches_3rdparty/")
        .expect("has_known_prefix で絞ってから呼ぶこと");
    let (first, rest) = split_first_path_segment(rest);

    // patches_3rdparty/<category>
    if rest.is_empty() {
        return (first, THIRD_PARTY_SORT_PRIORITY, "", "");
    }

    // patches_3rdparty/<category>/<patch>
    if !rest.contains('/') {
        return (first, THIRD_PARTY_SORT_PRIORITY, "", rest);
    }

    let (category, rest) = split_first_path_segment(rest);
    (category, THIRD_PARTY_SORT_PRIORITY, first, rest)
}

/// パス順ソート用に `(供給元の優先度, prefix を除いた残りのパス)` へ分解する。
pub(crate) fn path_sort_parts(path: &str) -> (u8, &str) {
    if let Some(rest) = path.strip_prefix("patches_factory/") {
        return (FACTORY_SORT_PRIORITY, rest);
    }

    let rest = path
        .strip_prefix("patches_3rdparty/")
        .expect("has_known_prefix で絞ってから呼ぶこと");
    (THIRD_PARTY_SORT_PRIORITY, rest)
}
