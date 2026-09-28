//! NORMAL の `t` で、音色 selector（`Ctrl+T` と同じ一覧）を入力欄なしで開く。
//!
//! 確定した音色は `Ctrl+T` と同じ経路でその track の init セルへ書き戻され、
//! 確定後は自動演奏を予約する。
//!
//! 試聴は、その track の init セルの effect chain を通して鳴らす。DAW で鳴る音と
//! 同じ文脈で音色を比べるため。鳴らすのはその meas のフレーズ全体。
//!
//! track の chain が手動 reverb でなければ、候補の音色に auto reverb を掛けて試聴する。
//! 確定したら、その 1 段を init セルの chain の auto reverb の段として書く（確定後の再生が
//! 試聴と同じ音になる）。chain のほかの段は残す。

use crossterm::event::KeyEvent;
use serde_json::Value;

use super::super::{DawApp, DawMode, CHORD_TRACK, FIRST_PLAYABLE_TRACK};
use cmrt_mml_overlay::cursor_notes::preview_note;
use cmrt_mml_overlay::line_play::line_events;
use cmrt_mml_overlay::{LivePatch, MmlOverlayAction, PatchAudition};
use cmrt_offline_render::EffectPlugins;
use cmrt_patch_select::auto_reverb::{AutoReverbRules, HostChain};
use cmrt_patch_select::{
    host_patch_catalog, AutoReverbHost, DirectPatchSelect, DirectPatchSelectRequest,
    DirectSelectOutcome, HostPatchCatalog, PatchAuditionAction, PatchAuditionContext,
    PatchCatalogNotice,
};

use super::INIT_MEASURE;
use crate::input::track_patch::PatchUpdateReason;

impl DawApp {
    /// カーソル track の音色 selector を開く。キーを消費したら true。
    pub(crate) fn open_direct_patch_select(&mut self) -> bool {
        if self.mode != DawMode::Normal {
            return false;
        }
        if self.editor.cursor_track < FIRST_PLAYABLE_TRACK
            && self.editor.cursor_track != CHORD_TRACK
        {
            self.append_log_line("音色選択は演奏トラックでのみ使用できます".to_string());
            return false;
        }
        let Some(target_track) = self.mml_overlay_target_track() else {
            self.append_log_line(
                "chord preview の演奏 track がありません。演奏 track の init に \"generate from chord track\" を設定してください",
            );
            return false;
        };
        // 試聴行は、音色を選ぶ track がその meas で DAW 上鳴らす MML。chord 行でも
        // chord 表記ではなく生成先 track の生成結果にする（Grid import の recipe は
        // chord の試聴文脈では再現できない）。init 列は音色 JSON なので空行。
        let preview_line = if self.editor.cursor_measure == INIT_MEASURE {
            String::new()
        } else {
            crate::mml::cell_preview_line(
                &self.editor.data,
                target_track,
                self.editor.cursor_measure,
            )
        };
        let HostPatchCatalog {
            catalog,
            patch_role_index,
            load_measurements,
        } = host_patch_catalog(&self.patch_load.lock().unwrap());
        let patch = self.track_patch_name(target_track);
        let chain = self.mark_track_manual_reverb_if_detected(target_track);
        let request = DirectPatchSelectRequest {
            context: PatchAuditionContext {
                catalog,
                patch_role_index,
                initial_role: None,
                load_measurements,
                filter_presets: cmrt_history::load_mml_patch_filter_presets(),
                catalog_notes: self.mml_overlay_catalog_notes(),
                favorites: cmrt_history::favorite_patch_names(&self.patch_phrase_store),
            },
            patch: patch.clone(),
            // 演奏設定は `i` の入力欄と共通。閉じたら書き戻す。
            play_settings: self.mml_overlay.play_settings(),
            audition: measure_audition(&preview_line),
            auto_reverb: Some(AutoReverbHost {
                rules: load_auto_reverb_rules(&self.effect_plugins),
                effect_plugins: self.effect_plugins.clone(),
                chain,
            }),
        };
        let (select, opening) = DirectPatchSelect::open(request);
        // 一覧が Error / 空なら開かず、理由を log 行へ出す。
        let Some(opening) = opening else {
            let reason = select
                .notice()
                .map(PatchCatalogNotice::message)
                .unwrap_or_default();
            self.append_log_line(format!("音色一覧を開けませんでした: {reason}"));
            return true;
        };
        // 音源は MML overlay と同じ instance を借りる。先に DAW の演奏を止めて明け渡す。
        self.stop_play();
        // effect chain を載せるのは `Some` の間だけなので、prepare より先に入れる。
        self.direct_patch_select = Some(select);
        self.mode = DawMode::DirectPatchSelect;
        if let Some(sender) = &self.mml_overlay_sender {
            let command_id = sender.prepare(self.mml_overlay_live_patch(patch.as_deref()));
            self.mml_overlay.expect_sender_command(command_id);
        }
        self.apply_mml_overlay_action(opening.into());
        true
    }

    /// 開いている間、キーはすべて selector が取る。閉じたら NORMAL へ戻り、
    /// `Enter` の確定なら音色を init セルへ書き戻して自動演奏を予約する。
    pub(crate) fn handle_direct_patch_select_key_event(&mut self, key: KeyEvent) {
        // loader 完了とキーが同じ frame に来ても、古い Loading を見せない。
        self.sync_direct_patch_select_catalog();
        let Some(select) = self.direct_patch_select.as_mut() else {
            self.mode = DawMode::Normal;
            return;
        };
        let patch_before = select.patch().map(str::to_string);
        match select.handle_key(key) {
            DirectSelectOutcome::Continue => {}
            DirectSelectOutcome::Play(action) => self.apply_mml_overlay_action(action.into()),
            DirectSelectOutcome::SavePresets { presets, preview } => {
                self.apply_mml_overlay_action(MmlOverlayAction::SavePatchFilterPresets {
                    presets,
                    preview: preview.map(|action| Box::new(action.into())),
                });
            }
            DirectSelectOutcome::SaveAutoReverb { rules, preview } => {
                let settings = cmrt_history::AutoReverbSettings {
                    enabled: rules.enabled(),
                    rules: rules.to_saved(),
                };
                if let Err(error) = cmrt_history::save_auto_reverb_settings(&settings) {
                    self.append_log_line(format!("auto reverb の保存に失敗: {error}"));
                }
                if let Some(preview) = preview {
                    self.apply_mml_overlay_action(preview.into());
                }
            }
            DirectSelectOutcome::Closed { confirmed, restore } => {
                self.close_direct_patch_select(patch_before, confirmed, restore);
            }
        }
    }

    fn close_direct_patch_select(
        &mut self,
        patch_before: Option<String>,
        confirmed: bool,
        restore: Option<PatchAuditionAction>,
    ) {
        // 戻す音色にも effect chain を載せるので、selector を手放す前に流す。
        if let Some(restore) = restore {
            self.apply_mml_overlay_action(restore.into());
        }
        let Some(select) = self.direct_patch_select.take() else {
            return;
        };
        self.mml_overlay
            .set_restored_play_settings(select.play_settings());
        if let Some(sender) = &self.mml_overlay_sender {
            let command_id = sender.stop();
            self.mml_overlay.expect_sender_command(command_id);
        }
        self.mode = DawMode::Normal;
        if confirmed {
            let patch_after = select.patch().map(str::to_string);
            let auto_reverb = select.confirmed_auto_reverb_stage().cloned();
            self.write_direct_confirmed_patch(patch_before, patch_after, auto_reverb);
            self.reserve_auto_play_after_render();
        }
    }

    /// 確定した音色を init セルへ書く。chain の auto reverb の段も `auto_reverb` に書き直す
    /// （dry・off なら外す）。手動 reverb の chain は触らない。
    fn write_direct_confirmed_patch(
        &mut self,
        patch_before: Option<String>,
        patch_after: Option<String>,
        auto_reverb: Option<Value>,
    ) {
        let (Some(track), Some(patch_name)) = (self.mml_overlay_target_track(), &patch_after)
        else {
            self.write_confirmed_patch(patch_before, patch_after);
            return;
        };
        let chain = self.track_host_chain(track);
        if chain.is_manual_reverb() {
            self.write_confirmed_patch(patch_before, patch_after);
            return;
        }
        let patch_filter_query = self.track_patch_filter_query(track);
        self.apply_patch_name_and_host_chain_to_track_init(
            track,
            patch_name,
            patch_filter_query.as_deref(),
            &chain,
            auto_reverb.as_ref(),
            PatchUpdateReason::MmlOverlay,
        );
    }

    /// track の init セルの effect chain と、auto reverb の扱い。
    fn track_host_chain(&self, track: usize) -> HostChain {
        crate::mml::effect_chain::init_cell_host_chain(
            &self.editor.data[track][INIT_MEASURE],
            self.effect_plugins.catalog(),
        )
    }

    /// track の chain に手動 reverb を見つけたら、init セルの auto reverb の控えを手動 reverb の
    /// 印に替える。返すのは track の chain。
    fn mark_track_manual_reverb_if_detected(&mut self, track: usize) -> HostChain {
        let chain = self.track_host_chain(track);
        if !chain.newly_detected_manual_reverb() {
            return chain;
        }
        let current_init = &self.editor.data[track][INIT_MEASURE];
        let next_init =
            crate::mml::effect_chain::init_cell_with_host_chain(current_init, &chain, None);
        self.apply_track_init_cell(track, next_init, PatchUpdateReason::MmlOverlay);
        self.append_log_line(format!(
            "auto reverb: {} の chain に手動 reverb を見つけたので manual reverb の印を書いた",
            crate::tracks::track_label(track)
        ));
        self.track_host_chain(track)
    }

    /// 一覧が Loading のまま開いていれば、loader の結果で差し替える。毎フレーム呼ぶ。
    pub(super) fn sync_direct_patch_select_catalog(&mut self) {
        if !self
            .direct_patch_select
            .as_ref()
            .is_some_and(DirectPatchSelect::is_waiting_for_catalog)
        {
            return;
        }
        let HostPatchCatalog {
            catalog,
            patch_role_index,
            load_measurements,
        } = host_patch_catalog(&self.patch_load.lock().unwrap());
        if let Some(select) = self.direct_patch_select.as_mut() {
            select.sync_catalog(catalog, patch_role_index, load_measurements);
        }
    }

    /// 試聴の余韻を消す対象か。`i` の入力欄の `Ctrl+T` と `t` のどちらでも、selector が
    /// 開いている間だけ。
    pub(super) fn is_patch_selector_open(&self) -> bool {
        self.mml_overlay.is_patch_select_open()
            || self
                .direct_patch_select
                .as_ref()
                .is_some_and(DirectPatchSelect::is_select_open)
    }

    /// sender へ渡す音色。`t` で開いている間だけ、track の effect chain を載せる。
    /// chain の auto reverb の段は、その音色に掛かる auto reverb に入れ替える。
    pub(super) fn mml_overlay_live_patch(&self, patch: Option<&str>) -> LivePatch {
        let Some(select) = self.direct_patch_select.as_ref() else {
            return LivePatch::new(patch);
        };
        let auto_reverb = patch.and_then(|patch| select.auto_reverb_stage(patch));
        let chain = self
            .mml_overlay_target_track()
            .map(|track| {
                self.track_host_chain(track)
                    .with_auto_reverb(auto_reverb.as_ref())
            })
            .unwrap_or_else(|| auto_reverb.into_iter().collect());
        if chain.is_empty() {
            return LivePatch::new(patch);
        }
        LivePatch::with_effect_chain(patch, &Value::Array(chain).to_string())
    }
}

/// selector の試聴で鳴らすもの。DAW の試聴の仕様は「その meas が演奏するフレーズ」。
/// フレーズが無い（init 列・空の meas）ときは、音色そのものを聴けるよう試聴用の 1 音。
fn measure_audition(preview_line: &str) -> Option<PatchAudition> {
    let (_, performance) = line_events(preview_line);
    if !performance.is_silent() {
        return Some(PatchAudition::Line(performance));
    }
    preview_note().map(PatchAudition::Notes)
}

/// 保存済みの auto reverb の設定。保存が無い・読めないときは既定のルール。
fn load_auto_reverb_rules(effect_plugins: &EffectPlugins) -> AutoReverbRules {
    cmrt_history::load_auto_reverb_settings()
        .map(|settings| {
            AutoReverbRules::from_saved(settings.enabled, &settings.rules, effect_plugins.catalog())
        })
        .unwrap_or_default()
}

#[cfg(test)]
pub(super) mod tests;
