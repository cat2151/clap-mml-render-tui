//! patch selector 内だけで使う、patch name の絞り込み状態と派生カテゴリ。

use cmrt_surge_patches::PatchCategory;
use ratatui_textarea::TextArea;

use super::PatchSelector;

pub(super) const ALL_CATEGORIES_NAME: &str = "全カテゴリ";

impl PatchSelector {
    pub(crate) fn query(&self) -> &str {
        &self.query
    }

    pub(crate) fn query_textarea(&self) -> &TextArea<'static> {
        &self.query_textarea
    }

    pub(crate) fn has_query(&self) -> bool {
        self.query.split_whitespace().next().is_some()
    }

    pub(crate) fn filter_visible(&self) -> bool {
        self.filter_active || self.has_query()
    }

    pub(super) fn start_filter_input(&mut self) {
        self.query_before_input = self.query.clone();
        self.category_cursor_before_input = self.category_cursor;
        self.patch_cursor_before_input = self.patch_cursor;
        self.query_textarea = cmrt_tui_core::text_input::new_single_line_textarea(&self.query);
        self.filter_active = true;
    }

    pub(super) fn confirm_filter_input(&mut self) {
        self.filter_active = false;
    }

    pub(super) fn cancel_filter_input(&mut self) {
        self.query = self.query_before_input.clone();
        self.query_textarea = cmrt_tui_core::text_input::new_single_line_textarea(&self.query);
        self.rebuild_categories();
        self.category_cursor = self
            .category_cursor_before_input
            .min(self.categories.len().saturating_sub(1));
        self.patch_cursor = self
            .patch_cursor_before_input
            .min(self.selected_category().patches.len().saturating_sub(1));
        self.filter_active = false;
    }

    pub(super) fn sync_filter_textarea(&mut self) {
        cmrt_tui_core::text_input::sync_single_line_textarea(&mut self.query_textarea, &self.query);
    }

    pub(super) fn apply_filter_key(&mut self, key: crossterm::event::KeyEvent) {
        if !cmrt_tui_core::text_input::apply_key_event_to_textarea(&mut self.query_textarea, key) {
            return;
        }
        self.query = cmrt_tui_core::text_input::textarea_value(&self.query_textarea);
        self.rebuild_categories();
        self.category_cursor = 0;
        self.patch_cursor = 0;
    }

    fn rebuild_categories(&mut self) {
        let terms = query_terms(&self.query);
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
                    .filter(|patch| patch_name_matches(patch, &terms))
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

    pub(super) fn select_random_filtered_patch(&mut self) {
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

fn query_terms(query: &str) -> Vec<String> {
    query
        .split_whitespace()
        .map(|term| term.to_lowercase())
        .collect()
}

fn patch_name_matches(path: &str, terms: &[String]) -> bool {
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
        assert!(patch_name_matches(
            "Instrument/Soft Strum.fxp",
            &query_terms("STRUM soft")
        ));
        assert!(!patch_name_matches(
            "Strum/Soft Pad.fxp",
            &query_terms("strum")
        ));
        assert!(!patch_name_matches(
            "Instrument/Everything.fxp",
            &query_terms("fxp")
        ));
    }
}
