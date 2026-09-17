//! 正規表現による patch 表示パスの絞り込み。
//!
//! 条件のコンパイルとマッチ規則そのものは `cmrt_tui_core::text_filter` が単一ソース。
//! ここは「patch のどのフィールドを条件に晒すか」だけを決める薄い層。

use cmrt_tui_core::text_filter;
use regex::Regex;

use crate::PatchCatalogEntry;

/// 動的な手入力は、事前検索済みの候補内だけを絞り込む。
pub fn filter_candidates(
    all: &[PatchCatalogEntry],
    candidates: &[usize],
    condition: &str,
) -> Result<Vec<usize>, String> {
    let required = text_filter::compile_condition(condition)?;
    Ok(candidates
        .iter()
        .copied()
        .filter(|index| condition_matches(&required, &all[*index]))
        .collect())
}

fn condition_matches(condition: &[Regex], patch: &PatchCatalogEntry) -> bool {
    let mut fields = vec![patch.normalized_display()];
    if let Some(category) = patch.normalized_selector_category() {
        fields.push(category);
    }
    text_filter::matches_any_field(condition, &fields)
}

pub(super) fn is_valid_condition(condition: &str) -> bool {
    text_filter::is_valid_condition(condition)
}
