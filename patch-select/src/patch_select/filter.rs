//! 正規表現による patch 表示パスの絞り込み。
//!
//! 条件のコンパイルとマッチ規則そのものは `cmrt_tui_core::text_filter` が単一ソース。
//! ここは「patch のどのフィールドを条件に晒すか」と、patch にしか無い plugin 条件を決める層。
//!
//! plugin 条件（`plugin:floe` / `-plugin:dexed`）の文法は Role 分類と共有する（[`PluginTerm`]）。
//! solo 同士は OR（AND では 1 音色が複数 plugin に属さないので必ず空になる）、mute は全部を除く。

use cmrt_tui_core::text_filter;
use regex::Regex;

use crate::PatchCatalogEntry;

pub(super) use cmrt_patches::{plugin_slug, PluginTerm};

fn plugin_is(patch: &PatchCatalogEntry, slug: &str) -> bool {
    plugin_slug(patch.plugin_sort_key()) == plugin_slug(slug)
}

struct Condition<'a> {
    plugins: Vec<PluginTerm<'a>>,
    regexes: Vec<Regex>,
}

fn compile(condition: &str) -> Result<Condition<'_>, String> {
    let mut plugins = Vec::new();
    let mut rest = Vec::new();
    for term in condition.split_whitespace() {
        match PluginTerm::parse(term) {
            Some(plugin) => plugins.push(plugin),
            None => rest.push(term),
        }
    }
    Ok(Condition {
        plugins,
        regexes: text_filter::compile_condition(&rest.join(" "))?,
    })
}

/// 動的な手入力は、事前検索済みの候補内だけを絞り込む。
pub fn filter_candidates(
    all: &[PatchCatalogEntry],
    candidates: &[usize],
    condition: &str,
) -> Result<Vec<usize>, String> {
    let condition = compile(condition)?;
    Ok(candidates
        .iter()
        .copied()
        .filter(|index| condition_matches(&condition, &all[*index]))
        .collect())
}

fn condition_matches(condition: &Condition<'_>, patch: &PatchCatalogEntry) -> bool {
    let plugins = &condition.plugins;
    let has_solo = plugins
        .iter()
        .any(|plugin| matches!(plugin, PluginTerm::Solo(_)));
    let soloed = plugins
        .iter()
        .any(|plugin| matches!(plugin, PluginTerm::Solo(slug) if plugin_is(patch, slug)));
    let muted = plugins
        .iter()
        .any(|plugin| matches!(plugin, PluginTerm::Mute(slug) if plugin_is(patch, slug)));
    if (has_solo && !soloed) || muted {
        return false;
    }
    let mut fields = vec![patch.normalized_display()];
    if let Some(category) = patch.normalized_selector_category() {
        fields.push(category);
    }
    fields.extend(patch.merged_names().iter().map(String::as_str));
    text_filter::matches_any_field(&condition.regexes, &fields)
}

pub(super) fn is_valid_condition(condition: &str) -> bool {
    compile(condition).is_ok()
}
