//! Chord Chart の `t`（Chord role）/ `Shift+T`（Bass role）で、音色 selector を入力欄なしで開く。
//!
//! 試聴（開いた直後・候補移動）は、その section の進行を role のパートで鳴らしたもの。
//! 確定した音色は role ごとの canonical patch へ入る。

use cmrt_patch_select::{
    DirectPatchSelect, DirectPatchSelectRequest, DirectSelectOutcome, HostPatchCatalog,
    PatchAuditionAction, PatchAuditionContext,
};
use cmrt_patches::PatchRole;
use crossterm::event::KeyEvent;

use super::super::mml_overlay::line_play::{
    locally_auto_voiced_bass_chord_chart_line_events, locally_auto_voiced_chord_chart_line_events,
};
use super::super::mml_overlay::{MmlOverlayAction, PatchAudition};
use super::super::TuiApp;

impl TuiApp<'_> {
    /// 指定 role の selector を開き、今の音色で section のフレーズを鳴らす。
    ///
    /// 一覧が Error / 空でも閉じずに理由を見せる（`Esc` で閉じる）。
    pub(super) fn open_chord_chart_patch_selector(&mut self, role: PatchRole) {
        let degrees = self
            .chord_chart
            .selected_preview_section()
            .map_or_else(String::new, |section| section.degrees.clone());
        let key_token = super::super::chord_chart_glue::key_token(&self.chord_chart.song.prefix)
            .map(str::to_owned);
        let HostPatchCatalog {
            catalog,
            patch_role_index,
            load_measurements,
        } = self.mml_overlay_patch_catalog_snapshot();
        let patch = self.chord_chart_role_patch(role);
        let (select, opening) = DirectPatchSelect::open(DirectPatchSelectRequest {
            context: PatchAuditionContext {
                catalog,
                patch_role_index,
                initial_role: Some(role),
                load_measurements,
                filter_presets: crate::history::load_mml_patch_filter_presets(),
                catalog_notes: self.mml_overlay_catalog_notes(),
            },
            patch: patch.clone(),
            // 演奏設定は `Ctrl+P` の入力欄と共通。閉じたら書き戻す。
            play_settings: self.mml_overlay.play_settings(),
            audition: chord_chart_section_audition(&degrees, key_token.as_deref(), role),
        });
        // 音源は MML overlay と同じ instance を借りるので、いまの画面の演奏を止めて明け渡す。
        self.stop_active_screen_playback();
        self.chord_chart_patch_select = Some((role, select));
        if let Some(sender) = &self.mml_overlay_sender {
            let command_id = sender.prepare(patch.as_deref());
            self.mml_overlay.expect_sender_command(command_id);
        }
        if let Some(opening) = opening {
            self.apply_mml_overlay_action(opening.into());
        }
    }

    /// 開いている間、キーはすべて selector が取る。
    pub(in crate::tui) fn handle_chord_chart_patch_select_key_event(&mut self, key: KeyEvent) {
        // loader 完了とキーが同じ frame に来ても、古い Loading を見せない。
        self.sync_chord_chart_patch_select_catalog();
        let Some((_, select)) = self.chord_chart_patch_select.as_mut() else {
            return;
        };
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
                self.close_chord_chart_patch_select(confirmed, restore);
            }
        }
    }

    /// 確定なら音色を role の canonical patch へ入れ、借りていた音源を画面へ返す。
    fn close_chord_chart_patch_select(
        &mut self,
        confirmed: bool,
        restore: Option<PatchAuditionAction>,
    ) {
        if let Some(restore) = restore {
            self.apply_mml_overlay_action(restore.into());
        }
        let Some((role, select)) = self.chord_chart_patch_select.take() else {
            return;
        };
        self.mml_overlay
            .set_restored_play_settings(select.play_settings());
        if confirmed {
            let patch = select.patch().map(str::to_owned);
            match role {
                PatchRole::Chord => self.chord_chart_patch = patch,
                PatchRole::Bass => self.chord_chart_bass_patch = patch,
                _ => {}
            }
        }
        self.save_history_state();
        if let Some(sender) = &self.mml_overlay_sender {
            let command_id = sender.stop();
            self.mml_overlay.expect_sender_command(command_id);
        }
        // Chord Chart はここで自動 preview しない。
        self.resume_active_screen_playback();
    }

    /// 一覧が Loading のまま開いていれば、loader の結果で差し替える。毎フレーム呼ぶ。
    pub(super) fn sync_chord_chart_patch_select_catalog(&mut self) {
        if !self
            .chord_chart_patch_select
            .as_ref()
            .is_some_and(|(_, select)| select.is_waiting_for_catalog())
        {
            return;
        }
        let HostPatchCatalog {
            catalog,
            patch_role_index,
            load_measurements,
        } = self.mml_overlay_patch_catalog_snapshot();
        if let Some((_, select)) = self.chord_chart_patch_select.as_mut() {
            select.sync_catalog(catalog, patch_role_index, load_measurements);
        }
    }

    fn chord_chart_role_patch(&self, role: PatchRole) -> Option<String> {
        match role {
            PatchRole::Chord => self.chord_chart_patch.clone(),
            PatchRole::Bass => self.chord_chart_bass_patch.clone(),
            _ => None,
        }
    }
}

/// Chord Chart の音色 selector の試聴で鳴らすもの。section の進行をローカルに auto voice し、
/// Bass role なら Bass パートだけ、それ以外は和音。進行が空か解釈できなければ鳴らさない
/// （MML の試聴用 1 音は進行の音ではない）。
fn chord_chart_section_audition(
    degrees: &str,
    key_token: Option<&str>,
    role: PatchRole,
) -> Option<PatchAudition> {
    let (_, performance) = match role {
        PatchRole::Bass => {
            locally_auto_voiced_bass_chord_chart_line_events(degrees, key_token, None)
        }
        _ => locally_auto_voiced_chord_chart_line_events(degrees, key_token, None),
    };
    (!performance.is_silent()).then_some(PatchAudition::Line(performance))
}
