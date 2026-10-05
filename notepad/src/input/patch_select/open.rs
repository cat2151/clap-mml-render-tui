use cmrt_offline_render::EffectPlugins;
use cmrt_patch_select::auto_reverb::{AutoReverbRules, HostChain};
use cmrt_patch_select::{
    host_patch_catalog, AutoReverbHost, FilterGroup, HostPatchCatalog, PatchCatalogSnapshot,
    PatchSelect, PatchSelectRequest,
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
        let chain = self.mark_current_line_manual_reverb_if_detected();
        let manual_reverb = chain.is_manual_reverb();
        let Some(select) = PatchSelect::open(PatchSelectRequest {
            patches,
            current,
            user_presets: cmrt_history::load_mml_patch_filter_presets(),
            role_index: patch_role_index,
            initial_role: None,
            drum_kit_only: false,
            catalog_notes: self.catalog_notes.clone(),
            load_measurements,
            favorites,
            initial_query: self.current_line_patch_filter_query().unwrap_or_default(),
            auto_reverb: Some(AutoReverbHost {
                rules: load_auto_reverb_rules(&self.effect_plugins),
                effect_plugins: self.effect_plugins.clone(),
                chain,
            }),
        }) else {
            return;
        };
        Self::log_notepad_event(format!(
            "tone-select open patches={} favorites={favorite_count} role={} preset={:?} cursor={} selected={:?} manual_reverb={manual_reverb}",
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

    /// 現在行の chain に手動 reverb を見つけたら、行頭 JSON の auto reverb の控えを
    /// 手動 reverb の印に替える。返すのは現在行の chain。
    fn mark_current_line_manual_reverb_if_detected(&mut self) -> HostChain {
        let chain = self.current_line_host_chain();
        if !chain.newly_detected_manual_reverb() {
            return chain;
        }
        let Some(patch_name) = self
            .editor
            .lines
            .get(self.editor.cursor)
            .and_then(|line| Self::extract_patch_phrase(line))
            .map(|(patch_name, _)| patch_name)
        else {
            return chain;
        };
        let effects = super::effect_keys(&chain, None);
        self.replace_current_line_patch(&patch_name, &effects);
        Self::log_notepad_event(format!(
            "auto-reverb: 手動 reverb を検出したので manual reverb の印を書いた patch={patch_name:?}"
        ));
        self.current_line_host_chain()
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

/// 保存済みの auto reverb の設定。保存が無い・読めないときは既定のルール。
fn load_auto_reverb_rules(effect_plugins: &EffectPlugins) -> AutoReverbRules {
    cmrt_history::load_auto_reverb_settings()
        .map(|settings| {
            AutoReverbRules::from_saved(settings.enabled, &settings.rules, effect_plugins.catalog())
        })
        .unwrap_or_default()
}
