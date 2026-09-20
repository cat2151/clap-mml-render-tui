use super::{
    build_cell_mml_from_data, DawApp, DawPatchSelectPane, DawPlayState, FIRST_PLAYABLE_TRACK,
};

impl DawApp {
    fn patch_select_patch_name_for_selection(
        &self,
        focus: DawPatchSelectPane,
        cursor: usize,
    ) -> Option<String> {
        match focus {
            DawPatchSelectPane::Patches => self.overlays.patch_select.filtered.get(cursor).cloned(),
            DawPatchSelectPane::Favorites => {
                self.patch_select_favorite_items().get(cursor).cloned()
            }
        }
    }

    pub(super) fn patch_select_selected_patch_name(&self) -> Option<String> {
        let cursor = match self.overlays.patch_select.focus {
            DawPatchSelectPane::Patches => self.overlays.patch_select.cursor,
            DawPatchSelectPane::Favorites => self.overlays.patch_select.favorites_cursor,
        };
        self.patch_select_patch_name_for_selection(self.overlays.patch_select.focus, cursor)
    }

    pub(super) fn patch_select_target_measure(&self) -> usize {
        self.editor.cursor_measure.max(1).min(self.editor.measures)
    }

    /// cursor track の init セルの音色を `patch_name` に差し替えたもの。
    /// `filter_query` は Enter の確定だけ渡す（[`DawApp::init_cell_with_patch`]）。
    pub(super) fn patch_select_init_with_patch(
        &self,
        patch_name: &str,
        filter_query: Option<&str>,
    ) -> String {
        Self::init_cell_with_patch(
            &self.editor.data[self.editor.cursor_track][0],
            patch_name,
            filter_query,
        )
    }

    fn patch_select_preview_track_mmls(
        &self,
        focus: DawPatchSelectPane,
        cursor: usize,
    ) -> Option<(usize, Vec<String>)> {
        if self.editor.cursor_track < FIRST_PLAYABLE_TRACK {
            return None;
        }

        let selected_patch_name = self.patch_select_patch_name_for_selection(focus, cursor)?;
        let target_measure = self.patch_select_target_measure();
        let measure_index = target_measure.checked_sub(1)?;

        let mut preview_data = self.preview_grid_for_cursor_track();
        preview_data[FIRST_PLAYABLE_TRACK][0] =
            self.patch_select_init_with_patch(&selected_patch_name, None);
        crate::mml::fill_preview_fallback_phrase(
            &mut preview_data,
            FIRST_PLAYABLE_TRACK,
            target_measure,
        );

        let mut track_mmls = self.build_measure_track_mmls_for_measure(target_measure);
        track_mmls[self.editor.cursor_track] = build_cell_mml_from_data(
            &preview_data,
            self.editor.measures,
            FIRST_PLAYABLE_TRACK,
            target_measure,
        );
        Some((measure_index, track_mmls))
    }

    fn prefetch_patch_select_navigation_cache(&self, preferred_delta: Option<isize>) {
        let (item_count, cursor) = match self.overlays.patch_select.focus {
            DawPatchSelectPane::Patches => (
                self.overlays.patch_select.filtered.len(),
                self.overlays.patch_select.cursor,
            ),
            DawPatchSelectPane::Favorites => (
                self.patch_select_favorite_items().len(),
                self.overlays.patch_select.favorites_cursor,
            ),
        };
        let focus = self.overlays.patch_select.focus;
        self.prefetch_preview_navigation_cache(
            cursor,
            item_count,
            1,
            preferred_delta,
            |next_cursor| self.patch_select_preview_track_mmls(focus, next_cursor),
        );
    }

    pub(super) fn preview_selected_patch(&mut self) {
        self.preview_selected_patch_with_navigation_hint(None);
    }

    pub(super) fn preview_selected_patch_with_navigation_hint(
        &mut self,
        preferred_delta: Option<isize>,
    ) {
        if *self.playback.play_state.lock().unwrap() == DawPlayState::Playing {
            return;
        }

        let cursor = match self.overlays.patch_select.focus {
            DawPatchSelectPane::Patches => self.overlays.patch_select.cursor,
            DawPatchSelectPane::Favorites => self.overlays.patch_select.favorites_cursor,
        };
        let Some((measure_index, track_mmls)) =
            self.patch_select_preview_track_mmls(self.overlays.patch_select.focus, cursor)
        else {
            return;
        };

        self.prefetch_patch_select_navigation_cache(preferred_delta);

        if self.try_start_preview_with_track_mmls_for_test(measure_index, Some(track_mmls.clone()))
        {
            return;
        }

        self.start_preview_with_snapshot(measure_index, track_mmls, self.playback_track_gains());
    }
}
