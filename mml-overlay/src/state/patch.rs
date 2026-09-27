//! `Ctrl+T` の音色選択。
//!
//! 音色は入力欄のテキストと別に持つ。行が増えると行頭 JSON を全部の行へ書くことに
//! なって邪魔になるうえ、聴き比べたいのはフレーズのほうで音色は共通、という使い方が
//! 前提のため。選んだ音色は枠のタイトルにだけ出る。

use std::collections::BTreeMap;

use cmrt_patch_select::{AuditionMoment, PatchAudition, PatchCatalogNotice, PatchSelectOutcome};
use cmrt_tui_core::patch_load::PatchLoadMeasurement;
use crossterm::event::KeyEvent;

use super::{MmlOverlay, MmlOverlayAction, PatchCatalogSnapshot, PatchChange};

impl MmlOverlay<'_> {
    /// `Ctrl+T` で音色 selector を開く。catalog が Loading なら完了後の open を予約する。
    pub(crate) fn request_patch_select(&mut self) {
        self.patch_audition_select.request_select();
    }

    /// 開いた時点で Loading だった一覧を、host app の loader 完了時に差し替える。
    ///
    /// Loading 中に Ctrl+T が押されていれば、Ready になった同じタイミングで selector を
    /// 自動で開く。overlay の開き直しを要求しないことがこの同期 API の責務。
    pub fn sync_patch_catalog(
        &mut self,
        catalog: PatchCatalogSnapshot,
        patch_role_index: cmrt_patches::PatchRoleIndex,
        load_measurements: BTreeMap<String, PatchLoadMeasurement>,
    ) {
        if !self.open {
            return;
        }
        self.patch_audition_select
            .sync_catalog(catalog, patch_role_index, load_measurements);
    }

    /// host app が毎 frame の loader polling を Loading 中だけに絞るための問い合わせ。
    pub fn is_waiting_for_patch_catalog(&self) -> bool {
        self.open && self.patch_audition_select.is_waiting_for_catalog()
    }

    pub fn is_patch_select_open(&self) -> bool {
        self.patch_audition_select.is_select_open()
    }

    pub(crate) fn patch_catalog_notice(&self) -> Option<&PatchCatalogNotice> {
        self.patch_audition_select.notice()
    }

    pub(super) fn handle_patch_select_key(&mut self, key: KeyEvent) -> MmlOverlayAction {
        match self.patch_audition_select.handle_select_key(key) {
            PatchSelectOutcome::Continue => MmlOverlayAction::Continue,
            PatchSelectOutcome::Audition {
                moment: AuditionMoment::Candidate,
                patch,
            } => {
                let audition = self.candidate_audition();
                self.audition(audition, patch)
            }
            PatchSelectOutcome::Audition {
                moment: AuditionMoment::Replay,
                patch,
            } => self.play_current_line(patch),
            PatchSelectOutcome::SavePresets { presets, preview } => {
                let preview = preview.map(|patch| {
                    let audition = self.preset_audition();
                    Box::new(self.audition(audition, PatchChange::Switch(Some(patch))))
                });
                MmlOverlayAction::SavePatchFilterPresets { presets, preview }
            }
            // `Ctrl+T` の selector は auto reverb を渡さずに開くので、この outcome は来ない。
            PatchSelectOutcome::SaveAutoReverb { .. } => MmlOverlayAction::Continue,
            PatchSelectOutcome::Closed { restore, .. } => {
                restore.map_or(MmlOverlayAction::Continue, Into::into)
            }
        }
    }

    fn audition(&self, audition: Option<PatchAudition>, patch: PatchChange) -> MmlOverlayAction {
        self.patch_audition_select
            .audition_action(audition, patch)
            .into()
    }
}
