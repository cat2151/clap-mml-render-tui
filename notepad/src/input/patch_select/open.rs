use cmrt_patch_select::{
    host_patch_catalog, FilterGroup, HostPatchCatalog, PatchCatalogSnapshot, PatchSelect,
    PatchSelectRequest,
};

use crate::NotepadScreen;
use crate::{Mode, PatchLoadState};

impl<'a> NotepadScreen<'a> {
    pub(super) fn start_patch_select_with_initial_patch_name(
        &mut self,
        initial_patch_name: Option<&str>,
    ) {
        let (catalog, pairs) = {
            let state = self.patch_load_state.lock().unwrap();
            let PatchLoadState::Ready(snapshot) = &*state else {
                return;
            };
            if !snapshot.catalog_notes().is_empty() {
                self.catalog_notes = snapshot.catalog_notes().to_vec();
            }
            (host_patch_catalog(&state), snapshot.pairs().to_vec())
        };
        let HostPatchCatalog {
            catalog: PatchCatalogSnapshot::Ready(patches),
            patch_role_index,
            load_measurements,
        } = catalog
        else {
            return;
        };
        self.normalize_patch_phrase_store_for_available_patches(&pairs);
        let patch_order = pairs
            .into_iter()
            .map(|(patch_name, _)| patch_name)
            .collect::<Vec<_>>();
        if cmrt_history::sync_patch_favorite_order(&mut self.patch_phrase_store, &patch_order) {
            self.patch_phrase_store_dirty = true;
        }
        let current = initial_patch_name
            .map(|patch_name| {
                self.resolve_loaded_patch_name(patch_name)
                    .unwrap_or_else(|| patch_name.to_string())
            })
            .or_else(|| self.current_line_patch_name());
        let favorites = cmrt_history::favorite_patch_names(&self.patch_phrase_store);
        let favorite_count = favorites.len();
        let Some(select) = PatchSelect::open(PatchSelectRequest {
            patches,
            current,
            user_presets: cmrt_history::load_mml_patch_filter_presets(),
            role_index: patch_role_index,
            initial_role: None,
            catalog_notes: self.catalog_notes.clone(),
            load_measurements,
            favorites,
            initial_query: self.current_line_patch_filter_query().unwrap_or_default(),
        }) else {
            return;
        };
        Self::log_notepad_event(format!(
            "tone-select open patches={} favorites={favorite_count} role={} preset={:?} cursor={} selected={:?}",
            select.filtered_len(),
            FilterGroup::ALL[select.group_cursor()].label(),
            select.presets()[select.preset_cursor()].label,
            select.cursor(),
            select.selected()
        ));
        self.patch_select = Some(select);
        self.mode = Mode::PatchSelect;
        self.preview_selected_patch();
    }

    /// 選択中の音色 × 現在行のフレーズを favorite へ登録し、`★ Favorite` を作り直す。
    pub(super) fn add_selected_patch_phrase_favorite(&mut self) {
        let Some(patch_name) = self.patch_select_selected_patch_name() else {
            return;
        };
        let Some(phrase) = self.patch_select_current_phrase() else {
            return;
        };
        self.add_patch_phrase_favorite(patch_name, phrase);
        let favorites = cmrt_history::favorite_patch_names(&self.patch_phrase_store);
        if let Some(select) = self.patch_select.as_mut() {
            select.set_favorites(favorites);
        }
        self.preview_selected_patch();
    }
}
