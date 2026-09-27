//! NORMAL の `t` で、音色 selector（`Ctrl+T` と同じ一覧）を入力欄なしで開く。
//!
//! 確定した音色は `Ctrl+T` と同じ経路でその track の init セルへ書き戻され、
//! 確定後は自動演奏を予約する。
//!
//! 試聴は、その track の init セルの effect chain を通して鳴らす。DAW で鳴る音と
//! 同じ文脈で音色を比べるため。鳴らすのはその meas のフレーズ全体。

use crossterm::event::KeyEvent;

use super::super::{DawApp, DawMode, CHORD_TRACK, FIRST_PLAYABLE_TRACK};
use cmrt_mml_overlay::cursor_notes::preview_note;
use cmrt_mml_overlay::line_play::line_events;
use cmrt_mml_overlay::{LivePatch, MmlOverlayAction, PatchAudition};
use cmrt_patch_select::{
    host_patch_catalog, DirectPatchSelect, DirectPatchSelectRequest, DirectSelectOutcome,
    HostPatchCatalog, PatchAuditionAction, PatchAuditionContext, PatchCatalogNotice,
};

use super::INIT_MEASURE;

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
            self.write_confirmed_patch(patch_before, select.patch().map(str::to_string));
            self.reserve_auto_play_after_render();
        }
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
    pub(super) fn mml_overlay_live_patch(&self, patch: Option<&str>) -> LivePatch {
        if self.direct_patch_select.is_none() {
            return LivePatch::new(patch);
        }
        let chain = self
            .mml_overlay_target_track()
            .map(|track| {
                crate::mml::effect_chain::init_cell_effect_chain(
                    &self.editor.data[track][INIT_MEASURE],
                )
            })
            .unwrap_or_default();
        if chain.is_empty() {
            return LivePatch::new(patch);
        }
        LivePatch::with_effect_chain(patch, &serde_json::Value::Array(chain).to_string())
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

#[cfg(test)]
pub(super) mod tests;
