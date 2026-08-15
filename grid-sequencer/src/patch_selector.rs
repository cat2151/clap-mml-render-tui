//! PATCH 欄から開く、mouse/keyboard 共用の行単位 patch selector。
//!
//! ここは選択状態と、preview / 確定 / 取り消しの適用まで。入力のさばきは
//! [`input`]、画面上の当たり判定は [`layout`] にある。

use std::ops::Range;

use cmrt_realtime_play::PatchVoicing;
use cmrt_surge_patches::{group_patch_pairs_by_category, matches_role, PatchCategory, PatchRole};
use cmrt_tui_core::random::random_index;
use ratatui_textarea::TextArea;

use crate::{
    patch_bag::PatchBag, GridPatchLoad, GridSequencerContext, GridSequencerScreen, ListDirection,
    ARPEGGIO_ROW, BASS_ROW, CHORD_ROW,
};

mod filter;
mod input;
mod layout;

use layout::contains;
pub(crate) use layout::PatchSelectorLayout;

pub(crate) struct PatchSelector {
    pub(crate) instance: usize,
    source_categories: Vec<PatchCategory>,
    pub(crate) categories: Vec<PatchCategory>,
    pub(crate) category_cursor: usize,
    pub(crate) patch_cursor: usize,
    query: String,
    query_textarea: TextArea<'static>,
    query_before_input: String,
    category_cursor_before_input: usize,
    patch_cursor_before_input: usize,
    pub(crate) filter_active: bool,
    poly_only: bool,
    original_patch: Option<String>,
    previewed_patch: Option<String>,
    patch_random_before_open: bool,
    undo_before_open: crate::undo::UndoSnapshot,
}

impl PatchSelector {
    fn new(
        instance: usize,
        current_patch: Option<&str>,
        ctx: &GridSequencerContext<'_>,
        poly_only: bool,
        patch_random_before_open: bool,
        undo_before_open: crate::undo::UndoSnapshot,
    ) -> Option<Self> {
        if !ctx.patch_dirs_configured {
            return None;
        }
        let GridPatchLoad::Ready(pairs) = &ctx.patch_load else {
            return None;
        };
        let pairs = pairs
            .iter()
            .filter(|(patch, _)| {
                !poly_only || ctx.voicing.cached_voicing(patch) == Some(PatchVoicing::Poly)
            })
            .cloned()
            .collect::<Vec<_>>();
        let categories = group_patch_pairs_by_category(&pairs);
        if categories.is_empty() {
            return None;
        }
        let selected = current_patch.and_then(|current| {
            categories
                .iter()
                .enumerate()
                .find_map(|(category_index, category)| {
                    category
                        .patches
                        .iter()
                        .position(|patch| patch == current)
                        .map(|patch_index| (category_index, patch_index))
                })
        });
        Some(Self {
            instance,
            source_categories: categories.clone(),
            categories,
            category_cursor: selected.map_or(0, |(category, _)| category),
            patch_cursor: selected.map_or(0, |(_, patch)| patch),
            query: String::new(),
            query_textarea: cmrt_tui_core::text_input::new_single_line_textarea(""),
            query_before_input: String::new(),
            category_cursor_before_input: 0,
            patch_cursor_before_input: 0,
            filter_active: false,
            poly_only,
            original_patch: current_patch.map(str::to_string),
            previewed_patch: current_patch.map(str::to_string),
            patch_random_before_open,
            undo_before_open,
        })
    }

    pub(crate) fn selected_category(&self) -> &PatchCategory {
        &self.categories[self.category_cursor]
    }

    pub(crate) fn selected_patch(&self) -> Option<&str> {
        self.selected_category()
            .patches
            .get(self.patch_cursor)
            .map(String::as_str)
    }

    fn move_category(&mut self, delta: isize) {
        let next = move_cursor(self.category_cursor, delta, self.categories.len());
        if next != self.category_cursor {
            self.category_cursor = next;
            self.patch_cursor = 0;
        }
    }

    fn move_patch(&mut self, delta: isize) {
        self.patch_cursor = move_cursor(
            self.patch_cursor,
            delta,
            self.selected_category().patches.len(),
        );
    }

    fn select_category(&mut self, index: usize) {
        if index < self.categories.len() && index != self.category_cursor {
            self.category_cursor = index;
            self.patch_cursor = 0;
        }
    }

    fn select_patch(&mut self, index: usize) {
        if index < self.selected_category().patches.len() {
            self.patch_cursor = index;
        }
    }

    fn select_random_patch(&mut self) {
        if self.has_query() {
            self.select_random_filtered_patch();
            return;
        }
        let total = self
            .categories
            .iter()
            .map(|category| category.patches.len())
            .sum::<usize>();
        if total <= 1 {
            return;
        }
        let current = self.categories[..self.category_cursor]
            .iter()
            .map(|category| category.patches.len())
            .sum::<usize>()
            + self.patch_cursor;
        let Some(random) = random_index(total - 1) else {
            return;
        };
        let target = if random >= current {
            random + 1
        } else {
            random
        };
        let mut offset = 0;
        for (category, item) in self.categories.iter().enumerate() {
            let next = offset + item.patches.len();
            if target < next {
                self.category_cursor = category;
                self.patch_cursor = target - offset;
                return;
            }
            offset = next;
        }
    }

    pub(crate) fn category_range(&self, layout: &PatchSelectorLayout) -> Range<usize> {
        visible_range(
            self.categories.len(),
            self.category_cursor,
            usize::from(layout.category_list.height),
        )
    }

    pub(crate) fn patch_range(&self, layout: &PatchSelectorLayout) -> Range<usize> {
        visible_range(
            self.selected_category().patches.len(),
            self.patch_cursor,
            usize::from(layout.patch_list.height),
        )
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
        let chord_on = self.state.chord().is_some();
        // chord mode 中は行ごとに用途が決まっている。それ以外の行は Free（＝和音向きの
        // 音色を避ける）で引き、chord mode off なら全行が Free。
        // drum 行は chord mode の on/off に関わらず用途が決まっている。
        let role = match self.state.drum_role(instance) {
            Some(drum) => crate::patch_role::drum_patch_role(drum),
            None => match instance {
                CHORD_ROW if chord_on => PatchRole::Chord,
                BASS_ROW if chord_on => PatchRole::Bass,
                ARPEGGIO_ROW if chord_on => PatchRole::Arpeggio,
                _ => PatchRole::Free,
            },
        };
        let role_filter = ctx.role_filter(role);
        let filter = role_filter.filter();
        let voicing = ctx.poly_lookup();
        // 現在の patch も除外しない。袋の中身は「用途に合う音色の全体」で固定しておき、
        // 音色を替えるたびに候補が変わって袋が作り直されるのを避ける。
        let candidates = ctx
            .patches()
            .iter()
            .filter(|(display, lower)| matches_role(display, lower, &filter, &voicing))
            .map(|(display, _)| display.clone())
            .collect::<Vec<_>>();
        // 用途か候補が変わったら、袋も辿った履歴も作り直す。
        if !matches!(self.patch_bags.get(&instance), Some(bag) if bag.matches(role, &candidates)) {
            self.patch_bags.insert(
                instance,
                PatchBag::new(role, candidates, current.as_deref()),
            );
        }
        let Some(bag) = self.patch_bags.get_mut(&instance) else {
            return;
        };
        let Some(patch) = bag.advance(direction).map(str::to_string) else {
            return;
        };
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
        let Some(selector) = PatchSelector::new(
            instance,
            current,
            ctx,
            poly_only,
            patch_random_before_open,
            undo_before_open,
        ) else {
            return;
        };
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

    pub(crate) fn prepare_instance_patch(&self, instance: usize) {
        let Some(patch) = self
            .state
            .instances()
            .get(instance)
            .map(|item| item.patch.as_deref())
        else {
            return;
        };
        self.prepare_patch(instance, patch, "undo");
    }

    fn prepare_patch(&self, instance: usize, patch: Option<&str>, reason: &'static str) {
        if self.state.instances().get(instance).is_none() {
            return;
        }
        if let Some(sender) = &self.midi_sender {
            let instance_id = self.state.instance_id(instance);
            let request_id = sender.set_row_patch(instance, instance_id, patch, reason);
            crate::log_line(&format!(
                "grid-sequencer: patch-selector request={request_id} reason={reason} instance={} \
                 active_bank={} instance={instance_id} chord_index={} patch={patch:?}",
                instance + 1,
                self.state.bank(),
                self.state
                    .chord()
                    .map_or("-".to_string(), |chord| chord.index().to_string()),
            ));
        }
    }
}

fn patch_is_available(patch: &str, poly_only: bool, ctx: &GridSequencerContext<'_>) -> bool {
    ctx.patch_dirs_configured
        && ctx.patches().iter().any(|(display, _)| display == patch)
        && (!poly_only || ctx.voicing.cached_voicing(patch) == Some(PatchVoicing::Poly))
}

fn move_cursor(current: usize, delta: isize, len: usize) -> usize {
    current
        .saturating_add_signed(delta)
        .min(len.saturating_sub(1))
}

fn visible_range(total: usize, cursor: usize, height: usize) -> Range<usize> {
    let visible = height.min(total);
    let start = cursor
        .saturating_sub(visible / 2)
        .min(total.saturating_sub(visible));
    start..start + visible
}

#[cfg(test)]
mod tests;
