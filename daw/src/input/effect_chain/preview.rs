//! `x`（chain 一覧）/ `x` → `a`（追加 overlay）の preview。
//!
//! overlay の track だけを鳴らし、他 track は無音にする（patch select preview の
//! 全 track 版とは異なる）。経路は既存の offline-render preview
//! （[`DawApp::start_uncached_preview_with_snapshot`]）で、realtime 経路は使わない。
//!
//! `track_mmls` は effect chain を差し込んだ一時的な上書きで `editor.data` には
//! 反映されないため、`allow_cell_cache=true` の経路（[`DawApp::start_preview_with_snapshot`]）
//! を使うと、実際の内容を見ないグローバル `CellCache` の Ready 判定にヒットして
//! 元のレンダリング結果（chain 変更前の音）が再生されてしまう。
//!
//! 追加 overlay はカーソルが動くたびに preview し、patch select と同じく隣の候補を
//! 先読み render して overlay preview cache に入れておく（次の j/k で render を待たない）。

use serde_json::Value;

use super::{build_cell_mml_from_data, DawApp, DawPlayState, FIRST_PLAYABLE_TRACK};

impl DawApp {
    fn effect_chain_preview_target_measure(&self) -> usize {
        self.editor.cursor_measure.max(1).min(self.editor.measures)
    }

    /// overlay track だけ音量を持つ gain。mute 中でも overlay の track は鳴らす。
    fn effect_chain_preview_track_gains(&self) -> Vec<f32> {
        let track = self.overlays.effect_chain.track;
        let mut track_gains = vec![0.0; self.editor.tracks];
        track_gains[track] = cmrt_tui_core::mixer::volume_db_to_gain(self.track_volume_db(track));
        track_gains
    }

    /// `chain`（bypass 反映済み）を overlay track の init へ差し込んだ MML を、対象 track
    /// だけに立てた `track_mmls` にして返す。他 track は空文字列のまま。
    fn effect_chain_preview_track_mmls(&self, chain: &[Value]) -> (usize, Vec<String>) {
        let track = self.overlays.effect_chain.track;
        let target_measure = self.effect_chain_preview_target_measure();
        let measure_index = target_measure - 1;

        let mut preview_data = self.preview_grid_for_track(track);
        preview_data[FIRST_PLAYABLE_TRACK][0] =
            crate::mml::effect_chain::init_cell_with_effect_chain(
                &preview_data[FIRST_PLAYABLE_TRACK][0],
                chain,
            );
        crate::mml::fill_preview_fallback_phrase(
            &mut preview_data,
            FIRST_PLAYABLE_TRACK,
            target_measure,
        );

        let mut track_mmls = vec![String::new(); self.editor.tracks];
        track_mmls[track] = build_cell_mml_from_data(
            &preview_data,
            self.editor.measures,
            FIRST_PLAYABLE_TRACK,
            target_measure,
        );
        (measure_index, track_mmls)
    }

    /// `chain` で現在 meas. を overlay track だけで preview する。再生中は何もしない。
    pub(crate) fn preview_effect_chain(&mut self, chain: &[Value]) {
        if *self.playback.play_state.lock().unwrap() == DawPlayState::Playing {
            return;
        }
        let (measure_index, track_mmls) = self.effect_chain_preview_track_mmls(chain);
        let track_gains = self.effect_chain_preview_track_gains();

        if self.try_start_preview_with_track_mmls_for_test(measure_index, Some(track_mmls.clone()))
        {
            return;
        }
        self.start_uncached_preview_with_snapshot(
            measure_index,
            track_mmls,
            track_gains,
            self.measure_duration_samples(),
        );
    }

    /// 編集中の chain（bypass 反映済み）をそのまま preview する。
    pub(crate) fn preview_editing_effect_chain(&mut self) {
        let chain = self.overlays.effect_chain.chain.clone();
        self.preview_effect_chain(&chain);
    }

    /// 追加 overlay の list `index` の preset を編集中 chain の末尾に足した chain。
    /// list が空、または index が範囲外なら `None`。
    fn effect_chain_add_candidate_chain(&self, index: usize) -> Option<Vec<Value>> {
        let preset_index = *self.overlays.effect_chain.add.list.get(index)?;
        let stage = self
            .effect_plugins
            .catalog()
            .and_then(|catalog| catalog.presets().get(preset_index))
            .map(cmrt_core::AudioEffectPreset::json_element)?;
        let mut chain = self.overlays.effect_chain.chain.clone();
        chain.push(stage);
        Some(chain)
    }

    /// 追加 overlay の list カーソルの候補を preview する。list が空なら何もしない。
    /// `preferred_delta` は直前のカーソル移動の向き（先読みの偏らせ方に使う）。
    pub(crate) fn preview_effect_chain_add_candidate(&mut self, preferred_delta: Option<isize>) {
        let add = &self.overlays.effect_chain.add;
        let (cursor, item_count) = (add.list_cursor, add.list.len());
        let Some(chain) = self.effect_chain_add_candidate_chain(cursor) else {
            return;
        };
        if *self.playback.play_state.lock().unwrap() == DawPlayState::Playing {
            return;
        }

        self.prefetch_preview_navigation_cache_with_gains(
            self.effect_chain_preview_track_gains(),
            cursor,
            item_count,
            crate::overlays::PAGE_STEP.unsigned_abs(),
            preferred_delta,
            |index| {
                self.effect_chain_add_candidate_chain(index)
                    .map(|chain| self.effect_chain_preview_track_mmls(&chain))
            },
        );
        self.preview_effect_chain(&chain);
    }
}
