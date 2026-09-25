//! `x`（chain 一覧）/ `x` → `a`（追加 overlay）の preview。
//!
//! overlay の track だけを鳴らし、他 track は無音にする（patch select preview の
//! 全 track 版とは異なる）。overlay preview cache に同じ内容の音があればそれを鳴らし、
//! 無ければ MML overlay sender で LIVE（realtime play server の live instance）へ、
//! 音色と effect chain を載せて鳴らす。どちらも backend の設定には関係しない。
//!
//! 引くのは overlay preview cache だけで、グローバル `CellCache` は見ない。`track_mmls` は
//! effect chain を差し込んだ一時的な上書きで `editor.data` には反映されないため、
//! `CellCache` の Ready 判定は実際の内容を見ずにヒットし、chain 変更前の音を返す。
//!
//! 追加 overlay はカーソルが動くたびに preview し、patch select と同じく隣の候補を
//! 先読み render して overlay preview cache に入れておく（次の j/k は cache から鳴る）。
//!
//! 次の候補を鳴らす前に、LIVE で鳴っている前の候補（音色の release・effect の余韻）を
//! [`PREVIEW_FADE_OUT_MS`] で fadeout する。次の候補の準備を待たずに、移動を受けた時点で送る。
//! cache の音（rodio）は従来どおり即座に止める。

use serde_json::Value;

use super::{build_cell_mml_from_data, DawApp, DawPlayState, FIRST_PLAYABLE_TRACK};
use crate::preview::service::OfflinePreviewRequest;

/// 次の候補へ移るとき、LIVE で鳴っている前の候補を絞りきる長さ。
const PREVIEW_FADE_OUT_MS: u32 = 50;

impl DawApp {
    fn effect_chain_preview_target_measure(&self) -> usize {
        self.editor.cursor_measure.max(1).min(self.editor.measures)
    }

    /// overlay track だけ音量を持つ gain。mute 中でも overlay の track は鳴らす。
    pub(crate) fn effect_chain_preview_track_gains(&self) -> Vec<f32> {
        let track = self.overlays.effect_chain.track;
        let mut track_gains = vec![0.0; self.editor.tracks];
        track_gains[track] = cmrt_tui_core::mixer::volume_db_to_gain(self.track_volume_db(track));
        track_gains
    }

    /// `chain`（bypass 反映済み）を overlay track の init へ差し込んだ MML を、対象 track
    /// だけに立てた `track_mmls` にして返す。他 track は空文字列のまま。
    pub(crate) fn effect_chain_preview_track_mmls(&self, chain: &[Value]) -> (usize, Vec<String>) {
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
        let live_settled = self.fade_out_effect_chain_live_preview();
        self.preview_effect_chain_after_fade_out(chain, live_settled);
    }

    /// LIVE で鳴っている前の候補を fadeout し、準備を待っている候補を鳴らさせない。
    /// LIVE の音が片づいたら `true`。fadeout を送れなかったら `false`（止める必要がある）。
    fn fade_out_effect_chain_live_preview(&mut self) -> bool {
        let Some(sender) = &self.mml_overlay_sender else {
            return true;
        };
        match sender.fade_out_line(PREVIEW_FADE_OUT_MS) {
            Ok(_) => true,
            Err(error) => {
                self.append_log_line(format!("effect-chain-preview: fade-out error {error}"));
                false
            }
        }
    }

    /// `live_settled` は [`Self::fade_out_effect_chain_live_preview`] の結果。
    fn preview_effect_chain_after_fade_out(&mut self, chain: &[Value], live_settled: bool) {
        let (measure_index, track_mmls) = self.effect_chain_preview_track_mmls(chain);
        let track_gains = self.effect_chain_preview_track_gains();
        self.overlays.effect_chain.preview_error = None;
        self.overlays.effect_chain.live_preview_command = None;

        let request = OfflinePreviewRequest::current(
            measure_index,
            self.measure_duration_samples(),
            crate::preview::active_preview_tracks(&track_mmls, &track_gains),
            track_mmls,
            track_gains,
        );
        if self.render.preview_service().cached(&request).is_some() {
            // fadeout 済みの LIVE は止めない。全NoteOff は fadeout を段差で切る。
            if !live_settled {
                self.stop_mml_overlay_sender();
            }
            if self.try_start_preview_with_track_mmls_for_test(
                measure_index,
                Some(request.track_mmls.clone()),
            ) || self.start_overlay_cached_preview(&request)
            {
                return;
            }
        }
        let track = self.overlays.effect_chain.track;
        self.start_effect_chain_live_preview(&request.track_mmls[track]);
    }

    /// cache の無い試聴を LIVE で鳴らす。鳴っている offline の preview は止め、LIVE の前の
    /// 試聴は sender が次の行で差し替える（止めずに release させる）。
    fn start_effect_chain_live_preview(&mut self, mml: &str) {
        self.stop_offline_playback();
        let Some(sender) = &self.mml_overlay_sender else {
            self.report_effect_chain_preview_error(
                "LIVE（realtime play server）が無いため試聴できません".to_string(),
            );
            return;
        };
        match cmrt_mml_overlay::live_line(mml) {
            Ok(line) => {
                let command_id = sender.play_line(line.patch, line.program);
                self.overlays.effect_chain.live_preview_command = Some(command_id);
                self.append_log_line(format!(
                    "effect-chain-preview: live command_id={command_id}"
                ));
            }
            Err(error) => self.report_effect_chain_preview_error(error),
        }
    }

    fn report_effect_chain_preview_error(&mut self, error: String) {
        self.append_log_line(format!("effect-chain-preview: error {error}"));
        self.overlays.effect_chain.preview_error = Some(error);
    }

    /// LIVE の試聴の準備が失敗したら、その理由を overlay へ出す。毎フレーム呼ぶ。
    pub(crate) fn pump_effect_chain_live_preview(&mut self) {
        let Some(command_id) = self.overlays.effect_chain.live_preview_command else {
            return;
        };
        let Some(sender) = &self.mml_overlay_sender else {
            return;
        };
        let status = sender.status();
        if let Some(error) = status.prepare_error_for(command_id) {
            let error = format!("試聴の準備に失敗しました: {error}");
            self.overlays.effect_chain.live_preview_command = None;
            self.report_effect_chain_preview_error(error);
            return;
        }
        let settled = status.command_id() > command_id
            || status
                .line_playback()
                .is_some_and(|playback| playback.command_id() == command_id);
        if settled {
            self.overlays.effect_chain.live_preview_command = None;
        }
    }

    /// 編集中の chain（bypass 反映済み）をそのまま preview する。
    pub(crate) fn preview_editing_effect_chain(&mut self) {
        let chain = self.overlays.effect_chain.chain.clone();
        self.preview_effect_chain(&chain);
    }

    /// 追加 overlay の list `index` の preset を編集中 chain の末尾に足した chain。
    /// list が空、または index が範囲外なら `None`。
    pub(crate) fn effect_chain_add_candidate_chain(&self, index: usize) -> Option<Vec<Value>> {
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
        let candidate_preview = crate::performance_log::SlowOperation::with_context(
            "effect-chain-preview-candidate",
            format!(
                "focus={:?} cursor={cursor} item_count={item_count} preferred_delta={preferred_delta:?}",
                add.focus
            ),
        );
        let Some(chain) = self.effect_chain_add_candidate_chain(cursor) else {
            return;
        };
        if *self.playback.play_state.lock().unwrap() == DawPlayState::Playing {
            return;
        }
        // 先読みの依頼より先に、移動を受けた時点で前の候補を絞り始める。
        let live_settled = self.fade_out_effect_chain_live_preview();

        {
            let _slow = crate::performance_log::SlowOperation::new("effect-chain-preview-prefetch");
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
        }
        {
            let _slow = crate::performance_log::SlowOperation::new(
                "effect-chain-preview-current-candidate",
            );
            self.preview_effect_chain_after_fade_out(&chain, live_settled);
        }
        drop(candidate_preview);
    }
}
