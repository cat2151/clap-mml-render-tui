//! PATCH 欄から開く、mouse/keyboard 共用の行単位 patch selector。
//!
//! MML overlay の patch selector と同じ Role / Preset / 音色 の 3 pane。分類の源
//! （`PatchRoleIndex` と Preset 一覧）は overlay と共有し、開いたときの Role / Preset は
//! 行の用途に合わせる。ここは選択状態と、preview / 確定 / 取り消しの適用まで。
//! pane の移動は [`navigation`]、Regex 絞り込みは [`filter`]、入力のさばきは
//! [`input`]、画面上の当たり判定は [`layout`]、各 pane の表示範囲は [`scroll`] にある。

use std::{cell::Cell, collections::BTreeMap, sync::Arc, time::Instant};

use cmrt_mml_overlay::{
    host_patch_catalog, prepare_user_presets, sort_for_selector, FilterGroup, FilterPreset,
    PatchCatalogEntry, PatchCatalogSnapshot, PreparedPresets,
};
use cmrt_realtime_play::PatchVoicing;
use cmrt_tui_core::{
    patch_load::{PatchLoadMeasurement, PatchLoadState},
    text_input,
};
use ratatui_textarea::TextArea;

use crate::{
    patch_bag::PatchBag,
    patch_notice::{catalog_unavailable, PatchNotice, PatchUnavailable},
    patch_role::{selector_start, GridPatchPurpose},
    GridSequencerContext, GridSequencerScreen, ListDirection, CHORD_ROW,
};

mod filter;
mod input;
mod layout;
mod navigation;
mod scroll;

use layout::contains;
pub(crate) use layout::PatchSelectorLayout;
pub(crate) use navigation::PatchPaneFocus;

pub(crate) struct PatchSelector {
    pub(crate) instance: usize,
    /// poly 絞り込み済み・selector 順に整列済みの全音色。
    entries: Vec<PatchCatalogEntry>,
    presets: PreparedPresets,
    /// 今見えている一覧（`entries` への index）。
    filtered: Arc<[usize]>,
    focus: PatchPaneFocus,
    /// 各 pane の描画時 scroll。viewport の高さは描画時にしか分からないため、
    /// 範囲を求めるときに interior mutability で更新する。
    scroll_offsets: [Cell<usize>; 3],
    role_cursor: usize,
    preset_cursor: usize,
    pub(crate) patch_cursor: usize,
    /// Regex 欄。編集中なら未確定の値も入る。
    query: TextArea<'static>,
    /// `Enter` で最後に確定した絞り込み。編集中の `Esc` はここへ戻す。
    committed_query: String,
    filter_editing: bool,
    filter_error: Option<String>,
    /// Load 列に出す読み込み時間。未計測の音色は載っていない。
    load_measurements: BTreeMap<String, PatchLoadMeasurement>,
    poly_only: bool,
    original_patch: Option<String>,
    previewed_patch: Option<String>,
    patch_random_before_open: bool,
    undo_before_open: crate::undo::UndoSnapshot,
    /// 設定不足でカタログから外れたプラグインの案内。開いている間ずっと枠下に出す。
    catalog_notes: Vec<String>,
}

impl PatchSelector {
    fn new(
        instance: usize,
        purpose: GridPatchPurpose,
        current_patch: Option<&str>,
        ctx: &GridSequencerContext<'_>,
        poly_only: bool,
        patch_random_before_open: bool,
        undo_before_open: crate::undo::UndoSnapshot,
    ) -> Result<Self, PatchUnavailable> {
        if let Some(reason) = catalog_unavailable(ctx) {
            return Err(reason);
        }
        let host = host_patch_catalog(ctx.patch_load);
        let (PatchLoadState::Ready(snapshot), PatchCatalogSnapshot::Ready(mut entries)) =
            (ctx.patch_load, host.catalog)
        else {
            return Err(catalog_unavailable(ctx).unwrap_or(PatchUnavailable::NoPatches));
        };
        entries.retain(|entry| {
            !poly_only || ctx.voicing.cached_voicing(entry.display()) == Some(PatchVoicing::Poly)
        });
        if entries.is_empty() {
            // ここまで来たら一覧は空でない。消えたのは poly 絞り込みのせい。
            return Err(PatchUnavailable::NoPolyPatches);
        }
        sort_for_selector(&mut entries);
        let user_presets = prepare_user_presets(snapshot.role_presets().to_vec());
        let presets = PreparedPresets::build(&entries, &user_presets, &ctx.patch_roles)
            .expect("prepared user presets compile");
        let (group, drum) = selector_start(purpose);
        let role_cursor = FilterGroup::ALL
            .iter()
            .position(|candidate| *candidate == group)
            .expect("FilterGroup::ALL contains every group");
        let preset_cursor = drum
            .and_then(|drum| {
                presets
                    .for_role(role_cursor)
                    .iter()
                    .position(|preset| preset.pattern.as_deref() == Some(drum.pattern()))
            })
            .unwrap_or(0);
        let filtered = Arc::clone(&presets.for_role(role_cursor)[preset_cursor].matches);
        let patch_cursor = current_patch
            .and_then(|current| {
                filtered
                    .iter()
                    .position(|index| entries[*index].display() == current)
            })
            .unwrap_or(0);
        Ok(Self {
            instance,
            entries,
            presets,
            filtered,
            focus: PatchPaneFocus::Patches,
            scroll_offsets: [Cell::new(0), Cell::new(0), Cell::new(0)],
            role_cursor,
            preset_cursor,
            patch_cursor,
            query: text_input::new_single_line_textarea(""),
            committed_query: String::new(),
            filter_editing: false,
            filter_error: None,
            load_measurements: host.load_measurements,
            poly_only,
            original_patch: current_patch.map(str::to_string),
            previewed_patch: current_patch.map(str::to_string),
            patch_random_before_open,
            undo_before_open,
            catalog_notes: ctx.catalog_notes.to_vec(),
        })
    }

    /// 設定不足でカタログから外れたプラグインの案内。無ければ空。
    pub(crate) fn catalog_notes(&self) -> &[String] {
        &self.catalog_notes
    }

    pub(crate) fn focus(&self) -> PatchPaneFocus {
        self.focus
    }

    pub(crate) fn roles(&self) -> &[FilterGroup] {
        &FilterGroup::ALL
    }

    pub(crate) fn role_cursor(&self) -> usize {
        self.role_cursor
    }

    /// 選択中の Role の Preset 一覧。
    pub(crate) fn presets(&self) -> &[FilterPreset] {
        self.presets.for_role(self.role_cursor)
    }

    pub(crate) fn preset_cursor(&self) -> usize {
        self.preset_cursor
    }

    /// 今見えている一覧の音色。
    pub(crate) fn filtered_entry(&self, index: usize) -> Option<&PatchCatalogEntry> {
        self.filtered.get(index).map(|index| &self.entries[*index])
    }

    pub(crate) fn filtered_len(&self) -> usize {
        self.filtered.len()
    }

    /// poly 絞り込み後の全音色数。
    pub(crate) fn total(&self) -> usize {
        self.entries.len()
    }

    pub(crate) fn load_measurement(&self, patch: &str) -> Option<&PatchLoadMeasurement> {
        self.load_measurements.get(patch)
    }

    pub(crate) fn selected_patch(&self) -> Option<&str> {
        self.filtered_entry(self.patch_cursor)
            .map(PatchCatalogEntry::display)
    }

    fn position_of(&self, patch: Option<&str>) -> Option<usize> {
        let patch = patch?;
        self.filtered
            .iter()
            .position(|index| self.entries[*index].display() == patch)
    }
}

impl GridSequencerScreen {
    /// PATCH名上のwheelで、そのinstanceの用途に合う patch list を1つ送る。
    ///
    /// 引く順序は [`PatchBag`] が持つ。下で次、上で前に聴いた音色へ戻る。
    pub(crate) fn cycle_instance_patch(
        &mut self,
        instance: usize,
        direction: ListDirection,
        ctx: &GridSequencerContext<'_>,
    ) {
        let Some(item) = self.state.instances().get(instance) else {
            return;
        };
        let current = item.patch.clone();
        let purpose = self.row_patch_purpose(instance);
        // 現在の patch も除外しない。袋の中身は「用途に合う音色の全体」で固定しておき、
        // 音色を替えるたびに候補が変わって袋が作り直されるのを避ける。
        let candidates = self.patch_candidates_for_row(instance, ctx);
        // 用途か候補が変わったら、袋も辿った履歴も作り直す。
        if !matches!(self.patch_bags.get(&instance), Some(bag) if bag.matches(purpose, &candidates))
        {
            self.patch_bags.insert(
                instance,
                PatchBag::new(purpose, candidates, current.as_deref()),
            );
        }
        let Some(bag) = self.patch_bags.get_mut(&instance) else {
            return;
        };
        let Some(patch) = bag.advance(direction).map(str::to_string) else {
            // 袋が空。ここで黙って戻ると wheel も「回しても無反応」にしか見えない。
            let reason = catalog_unavailable(ctx).unwrap_or(PatchUnavailable::NoRolePatches);
            self.patch_notice = Some(PatchNotice::new(reason, Instant::now()));
            return;
        };
        self.patch_notice = None;
        let undo = self.capture_undo();
        if self.state.set_instance_patch(instance, patch.clone()) {
            self.begin_manual_edit(crate::CycleRandomItem::Patch);
            self.prepare_patch(instance, Some(&patch), "wheel-patch");
            self.commit_undo(undo);
        }
    }

    /// instance 番号の指す先が変わるとき（`t` キーの track 数切替）に袋を捨てる。
    pub(crate) fn reset_patch_bags(&mut self) {
        self.patch_bags.clear();
    }

    pub(crate) fn open_patch_selector(&mut self, instance: usize, ctx: &GridSequencerContext<'_>) {
        let undo_before_open = self.capture_undo();
        let patch_random_before_open = self.cycle_random.patch;
        let Some(current) = self
            .state
            .instances()
            .get(instance)
            .map(|item| item.patch.as_deref())
        else {
            return;
        };
        let poly_only = instance == CHORD_ROW && self.state.chord().is_some();
        let selector = match PatchSelector::new(
            instance,
            self.row_patch_purpose(instance),
            current,
            ctx,
            poly_only,
            patch_random_before_open,
            undo_before_open,
        ) {
            Ok(selector) => selector,
            // 開けないときに黙って戻ると、押しても無反応にしか見えない。理由を出す。
            Err(reason) => {
                self.patch_notice = Some(PatchNotice::new(reason, Instant::now()));
                return;
            }
        };
        self.patch_notice = None;
        self.patch_selector = Some(selector);
        // selector 内で試聴している patch を周回境界の自動抽選で上書きさせない。
        // Esc 等でキャンセルした場合は、開く前の設定へ戻す。
        self.begin_manual_edit(crate::CycleRandomItem::Patch);
    }

    fn preview_patch_selection(&mut self, ctx: &GridSequencerContext<'_>) {
        let Some((instance, patch, poly_only, already_previewed)) =
            self.patch_selector.as_ref().and_then(|selector| {
                selector.selected_patch().map(|patch| {
                    (
                        selector.instance,
                        patch.to_string(),
                        selector.poly_only,
                        selector.previewed_patch.as_deref() == Some(patch),
                    )
                })
            })
        else {
            return;
        };
        if already_previewed || !patch_is_available(&patch, poly_only, ctx) {
            return;
        }
        self.prepare_patch(instance, Some(&patch), "preview");
        let Some(selector) = self.patch_selector.as_mut() else {
            return;
        };
        if selector.instance == instance && selector.selected_patch() == Some(patch.as_str()) {
            selector.previewed_patch = Some(patch);
        }
    }

    pub(crate) fn cancel_patch_selector(&mut self) {
        let Some(selector) = self.patch_selector.take() else {
            return;
        };
        if selector.previewed_patch != selector.original_patch {
            self.prepare_patch(
                selector.instance,
                selector.original_patch.as_deref(),
                "cancel-restore",
            );
        }
        self.set_cycle_random(
            crate::CycleRandomItem::Patch,
            selector.patch_random_before_open,
        );
    }

    fn apply_patch_selection(&mut self, ctx: &GridSequencerContext<'_>) {
        let Some(patch) = self
            .patch_selector
            .as_ref()
            .and_then(PatchSelector::selected_patch)
            .map(str::to_string)
        else {
            return;
        };
        let Some(selector) = self.patch_selector.take() else {
            return;
        };
        let instance = selector.instance;
        if !patch_is_available(&patch, selector.poly_only, ctx) {
            if selector.previewed_patch != selector.original_patch {
                self.prepare_patch(
                    instance,
                    selector.original_patch.as_deref(),
                    "invalid-restore",
                );
            }
            self.set_cycle_random(
                crate::CycleRandomItem::Patch,
                selector.patch_random_before_open,
            );
            return;
        }
        if self.state.set_instance_patch(instance, patch.clone()) {
            // preview の送信先 bank は、確定までの間に切り替わり得る。patch 名が同じ
            // というだけで省略せず、PATCH を据え置きに倒した後の active instance へ
            // 必ず積む。
            self.prepare_patch(instance, Some(&patch), "confirm");
        }
        // selector を開く操作から確定までを1操作として扱う。同じ patch を確定して
        // 音色だけ据え置いた場合も、PATCH random を OFF にした差分を undo 可能にする。
        self.commit_undo(selector.undo_before_open);
    }

    pub(crate) fn prepare_instance_patch(&mut self, instance: usize) {
        let Some(patch) = self
            .state
            .instances()
            .get(instance)
            .map(|item| item.patch.clone())
        else {
            return;
        };
        self.prepare_patch(instance, patch.as_deref(), "undo");
    }

    /// 行の音色をロードし、ロードで消える音を同じ残り長で鳴らし直す。
    ///
    /// sender の queue は直列なので、ロード要求の直後に積んだ note はロード完了後に
    /// 届く。鳴らし直す音の決め方は [`GridState::reattack_instance_now`]。
    fn prepare_patch(&mut self, instance: usize, patch: Option<&str>, reason: &'static str) {
        if self.state.instances().get(instance).is_none() {
            return;
        }
        let instance_id = self.state.instance_id(instance);
        let request_id = self
            .midi_sender
            .as_ref()
            .map(|sender| sender.set_row_patch(instance, instance_id, patch, reason));
        let reattack = self.state.reattack_instance_now(instance, Instant::now());
        self.send_scheduled(&reattack);
        if let Some(request_id) = request_id {
            crate::log_line(&format!(
                "grid-sequencer: patch-selector request={request_id} reason={reason} instance={} \
                 active_bank={} instance={instance_id} chord_index={} patch={patch:?} \
                 reattack_notes={}",
                instance + 1,
                self.state.bank(),
                self.state
                    .chord()
                    .map_or("-".to_string(), |chord| chord.index().to_string()),
                // 1 音につき note off と note on の 2 message。
                reattack.len() / 2,
            ));
        }
    }
}

fn patch_is_available(patch: &str, poly_only: bool, ctx: &GridSequencerContext<'_>) -> bool {
    ctx.patch_dirs_configured
        && ctx.patches().iter().any(|(display, _)| display == patch)
        && (!poly_only || ctx.voicing.cached_voicing(patch) == Some(PatchVoicing::Poly))
}

#[cfg(test)]
mod tests;
