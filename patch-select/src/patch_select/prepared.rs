//! 共通PatchRoleIndexから、各Presetの検索済みindex列を一度だけ準備する。

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use cmrt_patches::{DrumPatchRole, PatchRole, PatchRoleIndex, PatchRoleInput};

use crate::PatchCatalogEntry;

use super::{
    filter::filter_candidates,
    presets::{presets_for, FilterGroup, FilterPreset},
};

/// Role ごとの Preset 一覧。Role=ALL の一覧には他 Role の Preset を qualify 表記で含める。
pub struct PreparedPresets {
    by_role: Vec<Vec<FilterPreset>>,
}

impl PreparedPresets {
    pub fn build(
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

    pub fn for_role(&self, role_index: usize) -> &[FilterPreset] {
        &self.by_role[role_index]
    }

    /// 各 Role の preset 1 を `★ Favorite` にする（既にあれば中身を差し替える）。
    ///
    /// 中身はその Role の `ALL` に含まれる favorite を `favorites` の順に並べたもの。
    /// Role=`ALL` 群へ他 Role の `★ Favorite` は連結しない。
    pub fn set_favorites(&mut self, all: &[PatchCatalogEntry], favorites: &[String]) {
        let index_of = all
            .iter()
            .enumerate()
            .map(|(index, patch)| (patch.display(), index))
            .collect::<HashMap<_, _>>();
        let mut seen = HashSet::new();
        let favorite_indices = favorites
            .iter()
            .filter_map(|patch| index_of.get(patch.as_str()).copied())
            .filter(|index| seen.insert(*index))
            .collect::<Vec<_>>();
        for presets in &mut self.by_role {
            let role_all = presets[0].matches.iter().copied().collect::<HashSet<_>>();
            let matches = favorite_indices
                .iter()
                .copied()
                .filter(|index| role_all.contains(index))
                .collect::<Arc<[usize]>>();
            let favorite = FilterPreset::favorite(presets[0].group, matches);
            match presets.get_mut(1) {
                Some(preset) if preset.is_favorite => *preset = favorite,
                _ => presets.insert(1, favorite),
            }
        }
    }

    /// 開いた直後に選ぶ `(Role, Preset)` の位置。`role` が無ければ `ALL`。
    /// `drum` はその部位の Preset を選び、見つからなければ Role の `ALL`。
    pub fn start_cursors(
        &self,
        role: Option<PatchRole>,
        drum: Option<DrumPatchRole>,
    ) -> (usize, usize) {
        self.start_cursors_for_patch(role, drum, None)
    }

    /// [`Self::start_cursors`] に加え、部位で決まらなければ `patch`（`all` への index）を
    /// matches に含む最初の Preset を選ぶ。`ALL` と `★ Favorite` はその候補にしない。
    pub fn start_cursors_for_patch(
        &self,
        role: Option<PatchRole>,
        drum: Option<DrumPatchRole>,
        patch: Option<usize>,
    ) -> (usize, usize) {
        let group = role
            .and_then(|role| {
                FilterGroup::ALL
                    .iter()
                    .position(|group| group.role() == Some(role))
            })
            .unwrap_or(0);
        let presets = self.for_role(group);
        let by_drum = drum.and_then(|drum| {
            presets
                .iter()
                .position(|preset| preset.pattern.as_deref() == Some(drum.pattern()))
        });
        let by_patch = || {
            // Role=`ALL` の Preset は他 Role の連結なので、Role が分からない音色は `ALL` のまま。
            patch.filter(|_| group != 0).and_then(|patch| {
                presets.iter().position(|preset| {
                    preset.pattern.is_some()
                        && !preset.is_favorite
                        && preset.matches.contains(&patch)
                })
            })
        };
        (group, by_drum.or_else(by_patch).unwrap_or(0))
    }
}

pub(super) fn build_role_index(
    all: &[PatchCatalogEntry],
    user_presets: &[(String, String)],
) -> PatchRoleIndex {
    PatchRoleIndex::build(
        all.iter().map(|patch| PatchRoleInput {
            display: patch.display(),
            normalized_display: patch.normalized_display(),
            selector_category: patch.selector_category(),
        }),
        user_presets,
    )
}
