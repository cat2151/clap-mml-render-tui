//! patch 一覧の並び替えとカテゴリ別グルーピング。

use std::cmp::Ordering;

use crate::naming::compare_normalized_patch_names_natural;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PatchSortOrder {
    Path,
    Category,
}

impl PatchSortOrder {
    pub fn toggle(self) -> Self {
        match self {
            Self::Path => Self::Category,
            Self::Category => Self::Path,
        }
    }

    pub fn status_label(self) -> &'static str {
        match self {
            Self::Path => "path",
            Self::Category => "category",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PatchCategory {
    pub name: String,
    pub patches: Vec<String>,
}

fn compare_patch_pairs_with_order(
    left_display: &str,
    left_lower: &str,
    right_display: &str,
    right_lower: &str,
    order: PatchSortOrder,
) -> Ordering {
    match order {
        PatchSortOrder::Path => {
            let left = clap_mml_play_server_core::patch_sort_metadata(left_lower);
            let right = clap_mml_play_server_core::patch_sort_metadata(right_lower);

            left.source_rank
                .cmp(&right.source_rank)
                .then_with(|| {
                    compare_normalized_patch_names_natural(&left.path_rest, &right.path_rest)
                })
                .then_with(|| compare_normalized_patch_names_natural(left_lower, right_lower))
                .then_with(|| left_display.cmp(right_display))
        }
        PatchSortOrder::Category => {
            let left = clap_mml_play_server_core::patch_sort_metadata(left_lower);
            let right = clap_mml_play_server_core::patch_sort_metadata(right_lower);

            compare_normalized_patch_names_natural(&left.category, &right.category)
                .then_with(|| left.source_rank.cmp(&right.source_rank))
                .then_with(|| compare_normalized_patch_names_natural(&left.vendor, &right.vendor))
                .then_with(|| {
                    compare_normalized_patch_names_natural(
                        &left.category_rest,
                        &right.category_rest,
                    )
                })
                .then_with(|| compare_normalized_patch_names_natural(left_lower, right_lower))
                .then_with(|| left_display.cmp(right_display))
        }
    }
}

pub fn sort_patch_pairs(pairs: &mut [(String, String)], order: PatchSortOrder) {
    pairs.sort_by(|(left_display, left_lower), (right_display, right_lower)| {
        compare_patch_pairs_with_order(left_display, left_lower, right_display, right_lower, order)
    });
}

/// factory と 3rdparty を同じカテゴリ名でまとめる。カテゴリの並びも patch の並びも
/// [`PatchSortOrder::Category`] に従う。
pub fn group_patch_pairs_by_category(pairs: &[(String, String)]) -> Vec<PatchCategory> {
    let mut sorted = pairs.to_vec();
    sort_patch_pairs(&mut sorted, PatchSortOrder::Category);

    let mut categories: Vec<(String, PatchCategory)> = Vec::new();
    for (display, lower) in sorted {
        let category_key = clap_mml_play_server_core::patch_sort_metadata(&lower).category;
        if let Some((last_key, category)) = categories.last_mut() {
            if *last_key == category_key {
                category.patches.push(display);
                continue;
            }
        }

        let category_name = clap_mml_play_server_core::patch_sort_metadata(&display).category;
        categories.push((
            category_key,
            PatchCategory {
                name: category_name,
                patches: vec![display],
            },
        ));
    }

    categories
        .into_iter()
        .map(|(_, category)| category)
        .collect()
}

#[cfg(test)]
mod tests;
