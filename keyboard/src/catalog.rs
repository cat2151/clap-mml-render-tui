//! keyboard 画面の音色一覧。MML overlay の patch selector と同じ Role / Preset / 音色 の 3 pane。
//!
//! 分類の源（`PatchRoleIndex` と Preset 一覧）は overlay と共有する。ここは
//! 「今どの一覧を見ていて、その中のどれが現在の音色か」だけを持つ。

mod navigation;

use std::collections::BTreeMap;

use ratatui::widgets::{ListState, TableState};

use cmrt_mml_overlay::{
    host_patch_catalog, prepare_user_presets, sort_for_selector, FilterGroup, FilterPreset,
    HostPatchCatalog, PatchCatalogEntry, PatchCatalogSnapshot, PreparedPresets,
};
use cmrt_patches::PatchRoleIndex;
use cmrt_tui_core::patch_load::{PatchLoadMeasurement, PatchLoadState};

use super::{KeyboardContext, KeyboardScreen};

pub use navigation::PatchPaneFocus;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum KeyboardPatchCatalogStatus {
    Loading,
    NotConfigured,
    Ready,
    Error(String),
}

pub struct KeyboardPatchCatalog {
    status: KeyboardPatchCatalogStatus,
    /// selector 順に整列済みの全音色。
    entries: Vec<PatchCatalogEntry>,
    presets: PreparedPresets,
    /// `presets` を作ったときのユーザー正規表現。snapshot 側と違えば作り直す。
    role_presets: Vec<(String, String)>,
    load_measurements: BTreeMap<String, PatchLoadMeasurement>,
    focus: PatchPaneFocus,
    role_cursor: usize,
    preset_cursor: usize,
    /// 現在の一覧内での位置。現在の音色が一覧に無ければ `None`。
    patch_cursor: Option<usize>,
    /// random 抽選の残り（現在の一覧内の index）。
    random_remaining: Vec<usize>,
    /// `random_remaining` を作ったときの `(role_cursor, preset_cursor)`。
    random_deck_key: Option<(usize, usize)>,
    role_list_state: ListState,
    preset_list_state: ListState,
    patch_table_state: TableState,
    /// 設定不足でカタログから外れたプラグインの案内。描画だけに使う。
    catalog_notes: Vec<String>,
}

impl Default for KeyboardPatchCatalog {
    fn default() -> Self {
        Self {
            status: KeyboardPatchCatalogStatus::Loading,
            entries: Vec::new(),
            presets: empty_presets(),
            role_presets: Vec::new(),
            load_measurements: BTreeMap::new(),
            focus: PatchPaneFocus::Patches,
            role_cursor: 0,
            preset_cursor: 0,
            patch_cursor: None,
            random_remaining: Vec::new(),
            random_deck_key: None,
            role_list_state: ListState::default(),
            preset_list_state: ListState::default(),
            patch_table_state: TableState::default(),
            catalog_notes: Vec::new(),
        }
    }
}

fn empty_presets() -> PreparedPresets {
    PreparedPresets::build(&[], &[], &PatchRoleIndex::default())
        .expect("builtin preset regular expressions compile")
}

impl KeyboardPatchCatalog {
    pub fn status(&self) -> &KeyboardPatchCatalogStatus {
        &self.status
    }

    pub(super) fn is_ready(&self) -> bool {
        self.status == KeyboardPatchCatalogStatus::Ready
    }

    pub(super) fn set_loading(&mut self) {
        self.status = KeyboardPatchCatalogStatus::Loading;
    }

    pub(super) fn set_not_configured(&mut self) {
        self.clear_with_status(KeyboardPatchCatalogStatus::NotConfigured);
    }

    pub(super) fn set_error(&mut self, error: String) {
        self.clear_with_status(KeyboardPatchCatalogStatus::Error(error));
    }

    fn clear_with_status(&mut self, status: KeyboardPatchCatalogStatus) {
        self.status = status;
        self.entries.clear();
        self.presets = empty_presets();
        self.role_presets.clear();
        self.load_measurements.clear();
        self.focus = PatchPaneFocus::Patches;
        self.role_cursor = 0;
        self.preset_cursor = 0;
        self.patch_cursor = None;
        self.clear_random_deck();
        self.role_list_state.select(None);
        self.preset_list_state.select(None);
        self.patch_table_state.select(None);
    }

    /// 一覧を最初から作る。focus は音色 pane、Role / Preset は `ALL` に戻す。
    pub(super) fn load(
        &mut self,
        host: HostPatchCatalog,
        role_presets: &[(String, String)],
        current_patch: Option<&str>,
    ) {
        if !self.replace_catalog(host, role_presets) {
            return;
        }
        self.focus = PatchPaneFocus::Patches;
        self.role_cursor = 0;
        self.preset_cursor = 0;
        self.patch_cursor = self.position_of(current_patch);
    }

    /// Preset 一覧だけ作り直す。focus と Role / Preset の位置は保つ。
    pub(super) fn reload_presets(
        &mut self,
        host: HostPatchCatalog,
        role_presets: &[(String, String)],
        current_patch: Option<&str>,
    ) {
        if !self.replace_catalog(host, role_presets) {
            return;
        }
        let last = self.presets().len().saturating_sub(1);
        self.preset_cursor = self.preset_cursor.min(last);
        self.patch_cursor = self.position_of(current_patch);
    }

    /// 一覧の中身を差し替える。Ready なら `true`。
    fn replace_catalog(
        &mut self,
        host: HostPatchCatalog,
        role_presets: &[(String, String)],
    ) -> bool {
        let mut entries = match host.catalog {
            PatchCatalogSnapshot::Loading => {
                self.set_loading();
                return false;
            }
            PatchCatalogSnapshot::Error(error) => {
                self.set_error(error);
                return false;
            }
            PatchCatalogSnapshot::Ready(entries) => entries,
        };
        sort_for_selector(&mut entries);
        let user_presets = prepare_user_presets(role_presets.to_vec());
        self.presets = PreparedPresets::build(&entries, &user_presets, &host.patch_role_index)
            .expect("prepared user presets compile");
        self.status = KeyboardPatchCatalogStatus::Ready;
        self.entries = entries;
        self.role_presets = role_presets.to_vec();
        self.load_measurements = host.load_measurements;
        self.clear_random_deck();
        true
    }

    /// 設定不足でカタログから外れたプラグインの案内。無ければ空。
    /// 文言は `cmrt_runtime::SkippedCatalogPlugin::notice_line` が単一ソース。
    pub fn catalog_notes(&self) -> &[String] {
        &self.catalog_notes
    }

    pub(super) fn set_catalog_notes(&mut self, notes: &[String]) {
        if self.catalog_notes != notes {
            self.catalog_notes = notes.to_vec();
        }
    }

    pub fn focus(&self) -> PatchPaneFocus {
        self.focus
    }

    pub fn roles(&self) -> &[FilterGroup] {
        &FilterGroup::ALL
    }

    pub fn role_cursor(&self) -> usize {
        self.role_cursor
    }

    /// 選択中の Role の Preset 一覧。
    pub fn presets(&self) -> &[FilterPreset] {
        self.presets.for_role(self.role_cursor)
    }

    pub fn preset_cursor(&self) -> usize {
        self.preset_cursor
    }

    /// 選択中の Preset が指す音色（`entries` への index 列）。
    fn list(&self) -> &[usize] {
        self.presets()
            .get(self.preset_cursor)
            .map(|preset| &*preset.matches)
            .unwrap_or(&[])
    }

    /// 選択中の Preset の音色一覧。
    pub fn patches(&self) -> impl ExactSizeIterator<Item = &PatchCatalogEntry> {
        self.list().iter().map(|index| &self.entries[*index])
    }

    pub fn selected_patch_index(&self) -> Option<usize> {
        self.patch_cursor
    }

    pub fn selected_patch(&self) -> Option<&str> {
        self.patch_cursor
            .and_then(|cursor| self.list().get(cursor))
            .map(|index| self.entries[*index].display())
    }

    pub fn load_measurement(&self, patch: &str) -> Option<&PatchLoadMeasurement> {
        self.load_measurements.get(patch)
    }

    fn position_of(&self, patch: Option<&str>) -> Option<usize> {
        let patch = patch?;
        self.list()
            .iter()
            .position(|index| self.entries[*index].display() == patch)
    }

    fn clear_random_deck(&mut self) {
        self.random_remaining.clear();
        self.random_deck_key = None;
    }

    pub fn sync_list_states(
        &mut self,
        role_page_size: usize,
        preset_page_size: usize,
        patch_page_size: usize,
    ) {
        let ready = self.is_ready();
        self.role_list_state
            .select(ready.then_some(self.role_cursor));
        *self.role_list_state.offset_mut() = scrolled_offset(
            self.role_list_state.offset(),
            ready.then_some(self.role_cursor),
            self.roles().len(),
            role_page_size,
        );
        self.preset_list_state
            .select(ready.then_some(self.preset_cursor));
        *self.preset_list_state.offset_mut() = scrolled_offset(
            self.preset_list_state.offset(),
            ready.then_some(self.preset_cursor),
            self.presets().len(),
            preset_page_size,
        );
        self.patch_table_state.select(self.patch_cursor);
        *self.patch_table_state.offset_mut() = scrolled_offset(
            self.patch_table_state.offset(),
            self.patch_cursor,
            self.list().len(),
            patch_page_size,
        );
    }

    pub fn role_list_state_mut(&mut self) -> &mut ListState {
        &mut self.role_list_state
    }

    pub fn preset_list_state_mut(&mut self) -> &mut ListState {
        &mut self.preset_list_state
    }

    pub fn patch_table_state_mut(&mut self) -> &mut TableState {
        &mut self.patch_table_state
    }
}

fn scrolled_offset(
    current: usize,
    cursor: Option<usize>,
    item_count: usize,
    page_size: usize,
) -> usize {
    let Some(cursor) = cursor else {
        return 0;
    };
    let visible = page_size.max(1).min(item_count.max(1));
    let max_offset = item_count.saturating_sub(visible);
    let current = current.min(max_offset);
    let next = if cursor < current {
        cursor
    } else if cursor >= current.saturating_add(visible) {
        cursor.saturating_add(1).saturating_sub(visible)
    } else {
        current
    };
    next.min(max_offset)
}

impl KeyboardScreen<'_> {
    pub fn sync_patch_catalog(&mut self, ctx: &KeyboardContext<'_>) {
        // 一覧が読めたかどうかと無関係に出す案内なので、下の早期 return より前に置く。
        self.state
            .patch_catalog
            .set_catalog_notes(ctx.catalog_notes);
        if !ctx.patch_dirs_configured {
            self.state.patch_catalog.set_not_configured();
            return;
        }
        let snapshot = match ctx.patch_load {
            PatchLoadState::Ready(snapshot) => snapshot,
            _ if self.state.patch_catalog.is_ready() => return,
            PatchLoadState::Loading => return self.state.patch_catalog.set_loading(),
            PatchLoadState::Err(error) => return self.state.patch_catalog.set_error(error.clone()),
        };
        let ready = self.state.patch_catalog.is_ready();
        if ready && self.state.patch_catalog.role_presets == snapshot.role_presets() {
            return;
        }
        let current_patch = self
            .state
            .patch()
            .and_then(|patch| cmrt_patches::resolve_display_patch_name(snapshot.pairs(), patch));
        let host = host_patch_catalog(ctx.patch_load);
        if ready {
            self.state.patch_catalog.reload_presets(
                host,
                snapshot.role_presets(),
                current_patch.as_deref(),
            );
        } else {
            self.state
                .patch_catalog
                .load(host, snapshot.role_presets(), current_patch.as_deref());
        }
    }

    pub(super) fn move_focus(&mut self, delta: isize, ctx: &KeyboardContext<'_>) {
        self.sync_patch_catalog(ctx);
        self.state.patch_catalog.move_focus(delta);
    }

    pub(super) fn move_focused_cursor(&mut self, delta: isize, ctx: &KeyboardContext<'_>) {
        self.sync_patch_catalog(ctx);
        let selected = self.state.patch_catalog.move_focused_cursor(delta);
        self.apply_patch_selection(selected, ctx);
    }

    pub(super) fn move_focused_to_start(&mut self, ctx: &KeyboardContext<'_>) {
        self.sync_patch_catalog(ctx);
        let selected = self.state.patch_catalog.move_focused_to_start();
        self.apply_patch_selection(selected, ctx);
    }

    pub(super) fn move_focused_to_end(&mut self, ctx: &KeyboardContext<'_>) {
        self.sync_patch_catalog(ctx);
        let selected = self.state.patch_catalog.move_focused_to_end();
        self.apply_patch_selection(selected, ctx);
    }

    pub(super) fn select_random_patch(&mut self, ctx: &KeyboardContext<'_>) {
        self.sync_patch_catalog(ctx);
        let selected = self.state.patch_catalog.select_random_patch();
        self.apply_patch_selection(selected, ctx);
    }

    fn apply_patch_selection(&mut self, selected: Option<String>, ctx: &KeyboardContext<'_>) {
        let Some(patch) = selected else {
            return;
        };
        let previous_patch = self.state.patch().map(str::to_string);
        if previous_patch.as_deref() == Some(patch.as_str()) {
            return;
        }
        // 自動送信系(周期modeやON状態)はpatch変更をまたいで維持する。
        // note offのみ送り、Ready復帰後にrefreshで現在値を新patchへ再送する。
        let note_offs = self.state.take_note_off_messages();
        self.state.patch = Some(patch.clone());
        let known_voicing = ctx.cached_voicing(Some(&patch));
        if let Some(sender) = &self.midi_sender {
            sender.set_patch(
                note_offs,
                previous_patch.as_deref(),
                Some(&patch),
                known_voicing,
            );
        }
    }
}

#[cfg(test)]
mod tests;
