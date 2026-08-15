//! patch selector 内だけで使う、filename stem 専用の patch name 検索。
//!
//! category、vendor、filename を含む表示パス全文検索とは検索範囲が異なる。
//! `Instrument/Soft Strum.fxp` を `strum` で発見しつつ、category 名だけが一致する
//! `Strum/Plain Pad.fxp` は結果に含めないため、表示パス検索とは共通化しない。

use cmrt_surge_patches::PatchCategory;
use ratatui_textarea::TextArea;

use super::PatchSelector;

pub(super) const ALL_CATEGORIES_NAME: &str = "全カテゴリ";

impl PatchSelector {
    pub(crate) fn name_query(&self) -> &str {
        &self.name_query
    }

    pub(crate) fn name_query_textarea(&self) -> &TextArea<'static> {
        &self.name_query_textarea
    }

    pub(crate) fn has_name_query(&self) -> bool {
        self.name_query.split_whitespace().next().is_some()
    }

    pub(crate) fn name_search_visible(&self) -> bool {
        self.name_search_active || self.has_name_query()
    }

    pub(super) fn start_name_search_input(&mut self) {
        self.name_query_before_input = self.name_query.clone();
        self.category_cursor_before_input = self.category_cursor;
        self.patch_cursor_before_input = self.patch_cursor;
        self.name_query_textarea =
            cmrt_tui_core::text_input::new_single_line_textarea(&self.name_query);
        self.name_search_active = true;
    }

    pub(super) fn confirm_name_search_input(&mut self) {
        self.name_search_active = false;
    }

    pub(super) fn cancel_name_search_input(&mut self) {
        self.name_query = self.name_query_before_input.clone();
        self.name_query_textarea =
            cmrt_tui_core::text_input::new_single_line_textarea(&self.name_query);
        self.rebuild_name_search_results();
        self.category_cursor = self
            .category_cursor_before_input
            .min(self.categories.len().saturating_sub(1));
        self.patch_cursor = self
            .patch_cursor_before_input
            .min(self.selected_category().patches.len().saturating_sub(1));
        self.name_search_active = false;
    }

    pub(super) fn sync_name_search_textarea(&mut self) {
        cmrt_tui_core::text_input::sync_single_line_textarea(
            &mut self.name_query_textarea,
            &self.name_query,
        );
    }

    pub(super) fn apply_name_search_key(&mut self, key: crossterm::event::KeyEvent) {
        if !cmrt_tui_core::text_input::apply_key_event_to_textarea(
            &mut self.name_query_textarea,
            key,
        ) {
            return;
        }
        self.name_query = cmrt_tui_core::text_input::textarea_value(&self.name_query_textarea);
        self.rebuild_name_search_results();
        self.category_cursor = 0;
        self.patch_cursor = 0;
    }

    fn rebuild_name_search_results(&mut self) {
        let terms = name_query_terms(&self.name_query);
        if terms.is_empty() {
            self.categories = self.source_categories.clone();
            return;
        }

        let matched = self
            .source_categories
            .iter()
            .filter_map(|category| {
                let patches = category
                    .patches
                    .iter()
                    .filter(|patch| patch_filename_stem_matches(patch, &terms))
                    .cloned()
                    .collect::<Vec<_>>();
                (!patches.is_empty()).then(|| PatchCategory {
                    name: category.name.clone(),
                    patches,
                })
            })
            .collect::<Vec<_>>();
        let all_patches = matched
            .iter()
            .flat_map(|category| category.patches.iter().cloned())
            .collect();
        self.categories = std::iter::once(PatchCategory {
            name: ALL_CATEGORIES_NAME.to_string(),
            patches: all_patches,
        })
        .chain(matched)
        .collect();
    }

    pub(super) fn select_random_name_search_result(&mut self) {
        let patches = &self.categories[0].patches;
        let total = patches.len();
        if total <= 1 {
            return;
        }
        let selected = self.selected_patch();
        let current = selected.and_then(|patch| patches.iter().position(|item| item == patch));
        let target = match current {
            Some(current) => {
                let Some(random) = cmrt_tui_core::random::random_index(total - 1) else {
                    return;
                };
                if random >= current {
                    random + 1
                } else {
                    random
                }
            }
            None => {
                let Some(random) = cmrt_tui_core::random::random_index(total) else {
                    return;
                };
                random
            }
        };
        self.category_cursor = 0;
        self.patch_cursor = target;
    }
}

fn name_query_terms(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(|term| term.to_lowercase())
        .collect()
}

fn patch_filename_stem_matches(path: &str, terms: &[String]) -> bool {
    let filename = path.rsplit(['/', '\\']).next().unwrap_or(path);
    let stem = filename
        .rsplit_once('.')
        .filter(|(stem, _)| !stem.is_empty())
        .map_or(filename, |(stem, _)| stem);
    let lower_stem = stem.to_lowercase();
    terms.iter().all(|term| lower_stem.contains(term))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_terms_against_only_the_filename_stem() {
        assert!(patch_filename_stem_matches(
            "Instrument/Soft Strum.fxp",
            &name_query_terms("STRUM soft")
        ));
        assert!(!patch_filename_stem_matches(
            "Strum/Soft Pad.fxp",
            &name_query_terms("strum")
        ));
        assert!(!patch_filename_stem_matches(
            "Instrument/Everything.fxp",
            &name_query_terms("fxp")
        ));
    }
}
