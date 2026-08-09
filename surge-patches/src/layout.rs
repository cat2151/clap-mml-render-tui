//! Surge XT の patch ディレクトリ構造の解析。
//!
//! `patches_factory/<category>/<patch>` と
//! `patches_3rdparty/<vendor>/<category>/<patch>`（vendor が無い形もある）から
//! カテゴリと vendor を取り出す。ここが Surge 依存の中心。

/// patch パスの先頭に来るディレクトリ名。factory を先に置き、ソート優先度と対応させる。
pub(crate) const PATCH_DIR_PREFIXES: [&str; 2] = ["patches_factory", "patches_3rdparty"];
pub(crate) const FACTORY_SORT_PRIORITY: u8 = 0;
pub(crate) const THIRD_PARTY_SORT_PRIORITY: u8 = 1;

fn split_first_path_segment(path: &str) -> (&str, &str) {
    path.split_once('/').unwrap_or((path, ""))
}

/// カテゴリ順ソート用に `(カテゴリ, 供給元の優先度, vendor, 残りのパス)` へ分解する。
pub(crate) fn patch_category_sort_parts(path: &str) -> (&str, u8, &str, &str) {
    let path = path.trim_matches('/');

    if let Some(rest) = path.strip_prefix("patches_factory/") {
        let (category, rest) = split_first_path_segment(rest);
        return (category, FACTORY_SORT_PRIORITY, "", rest);
    }

    if let Some(rest) = path.strip_prefix("patches_3rdparty/") {
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
        return (category, THIRD_PARTY_SORT_PRIORITY, first, rest);
    }

    let (category, rest) = split_first_path_segment(path);
    (category, FACTORY_SORT_PRIORITY, "", rest)
}

/// パス順ソート用に `(供給元の優先度, prefix を除いた残りのパス)` へ分解する。
pub(crate) fn patch_path_sort_parts(path: &str) -> (u8, &str) {
    let path = path.trim_matches('/');

    if let Some(rest) = path.strip_prefix("patches_factory/") {
        return (FACTORY_SORT_PRIORITY, rest);
    }

    if let Some(rest) = path.strip_prefix("patches_3rdparty/") {
        return (THIRD_PARTY_SORT_PRIORITY, rest);
    }

    (FACTORY_SORT_PRIORITY, path)
}

/// patch パスのカテゴリ階層（`patches_factory/<category>/` または
/// `patches_3rdparty/<vendor>/<category>/`）を返す。
pub fn patch_category(path: &str) -> &str {
    patch_category_sort_parts(path).0
}

/// patch がカテゴリ一覧のいずれかに属するか。大文字小文字は無視する。
/// カテゴリ一覧が空なら「絞り込まない」とみなして常に true。
///
/// Surge のカテゴリ名には `Bass`（単数）と `Basses`（複数）が併存するような表記ゆれが
/// あるが、正規化を足すと既存の config.toml の設定が壊れるので大文字小文字だけを無視する。
pub fn patch_matches_categories(path: &str, categories: &[String]) -> bool {
    if categories.is_empty() {
        return true;
    }
    let category = patch_category(path);
    categories
        .iter()
        .any(|allowed| allowed.eq_ignore_ascii_case(category))
}

#[cfg(test)]
mod tests;
