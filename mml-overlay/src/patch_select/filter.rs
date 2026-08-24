//! 正規表現による patch 表示パスの絞り込み。

use regex::{Regex, RegexBuilder};

use crate::PatchCatalogEntry;

/// 動的な手入力は、事前検索済みの候補内だけを絞り込む。
pub(super) fn filter_candidates(
    all: &[PatchCatalogEntry],
    candidates: &[usize],
    condition: &str,
) -> Result<Vec<usize>, String> {
    let required = compile_condition(condition)?;
    Ok(candidates
        .iter()
        .copied()
        .filter(|index| condition_matches(&required, &all[*index]))
        .collect())
}

fn condition_matches(condition: &[Regex], patch: &PatchCatalogEntry) -> bool {
    condition.iter().all(|regex| {
        regex.is_match(patch.normalized_display())
            || patch
                .normalized_selector_category()
                .is_some_and(|category| regex.is_match(category))
    })
}

pub(super) fn is_valid_condition(condition: &str) -> bool {
    compile_condition(condition).is_ok()
}

/// 空白区切りの各 term を正規表現にし、term 間を AND として扱う。
fn compile_condition(condition: &str) -> Result<Vec<Regex>, String> {
    condition
        .split_whitespace()
        .map(|term| {
            RegexBuilder::new(term)
                .case_insensitive(true)
                .build()
                .map_err(|error| error.to_string())
        })
        .collect()
}
