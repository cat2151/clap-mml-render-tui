//! Plugin-neutral facade over metadata produced by the play-server core.

/// patch パスのカテゴリ階層を返す。Surge なら `patches_factory/<category>/` または
/// `patches_3rdparty/<vendor>/<category>/`、カートリッジならカートリッジ名、
/// plugin adapter固有の配置規則をserver shared coreで解釈したカテゴリ名。
pub fn patch_category(path: &str) -> String {
    clap_mml_play_server_core::patch_sort_metadata(path).category
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
        .any(|allowed| allowed.eq_ignore_ascii_case(&category))
}

#[cfg(test)]
mod tests;
