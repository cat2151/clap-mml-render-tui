//! 共通PatchRoleIndexから、各Presetの検索済みindex列を一度だけ準備する。

use std::sync::Arc;

use cmrt_patches::PatchRoleIndex;

use crate::PatchCatalogEntry;

use super::{
    filter::filter_candidates,
    presets::{presets_for, FilterGroup, FilterPreset},
};

pub(super) struct PreparedPresets {
    by_role: Vec<Vec<FilterPreset>>,
}

impl PreparedPresets {
    pub(super) fn build(
        all: &[PatchCatalogEntry],
        user_presets: &[(String, String)],
        role_index: &PatchRoleIndex,
    ) -> Result<Self, String> {
        let all_indices = (0..all.len()).collect::<Arc<[usize]>>();
        let mut by_role = vec![Vec::new()];

        for group in FilterGroup::ALL.into_iter().skip(1) {
            let role = group.role().expect("non-ALL group has a PatchRole");
            let role_indices = all
                .iter()
                .enumerate()
                .filter(|(_, patch)| role_index.role_of(patch.display()) == Some(role))
                .map(|(index, _)| index)
                .collect::<Arc<[usize]>>();
            let mut presets = presets_for(group, user_presets);
            for preset in &mut presets {
                preset.matches = match preset.pattern.as_deref() {
                    None => Arc::clone(&role_indices),
                    Some(pattern) => filter_candidates(all, &role_indices, pattern)?.into(),
                };
            }
            by_role.push(presets);
        }

        let mut all_presets = presets_for(FilterGroup::All, user_presets);
        all_presets[0].matches = all_indices;
        all_presets.extend(
            by_role
                .iter()
                .skip(1)
                .flat_map(|presets| presets.iter().skip(1))
                .cloned()
                .map(FilterPreset::qualify_label),
        );
        by_role[0] = all_presets;
        Ok(Self { by_role })
    }

    pub(super) fn for_role(&self, role_index: usize) -> &[FilterPreset] {
        &self.by_role[role_index]
    }
}
