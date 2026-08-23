//! `Ctrl+T` の音色選択。
//!
//! 音色は入力欄のテキストと別に持つ。行が増えると行頭 JSON を全部の行へ書くことに
//! なって邪魔になるうえ、聴き比べたいのはフレーズのほうで音色は共通、という使い方が
//! 前提のため。選んだ音色は枠のタイトルにだけ出る。

use std::{collections::BTreeMap, time::Instant};

use cmrt_tui_core::patch_load::PatchLoadMeasurement;
use crossterm::event::KeyEvent;

use crate::cursor_notes::{notes_at_prefix, CursorNotes, PREVIEW_MML};
use crate::patch_select::{PatchSelect, PatchSelectAction};

use super::{MmlOverlay, MmlOverlayAction, PatchCatalogNotice, PatchCatalogSnapshot};

impl MmlOverlay<'_> {
    pub(super) fn open_patch_select(&mut self) {
        self.patch_catalog_notice = None;
        self.patch_select_requested = false;
        match &self.patch_catalog {
            PatchCatalogSnapshot::Loading => {
                self.patch_catalog_notice = Some(PatchCatalogNotice::Loading);
                self.patch_select_requested = true;
                crate::log_line(
                    "action=patch-select event=open result=waiting reason=catalog-loading"
                        .to_string(),
                );
            }
            PatchCatalogSnapshot::Error(error) => {
                self.patch_catalog_notice = Some(PatchCatalogNotice::Error(error.clone()));
                crate::log_line(format!(
                    "action=patch-select event=open result=blocked reason=catalog-error detail={error:?}"
                ));
            }
            PatchCatalogSnapshot::Ready(patches) if patches.is_empty() => {
                self.patch_catalog_notice = Some(PatchCatalogNotice::Empty);
                crate::log_line(
                    "action=patch-select event=open result=blocked reason=catalog-empty"
                        .to_string(),
                );
            }
            PatchCatalogSnapshot::Ready(patches) => {
                let count = patches.len();
                self.patch_select = PatchSelect::open(
                    patches.clone(),
                    self.patch.as_deref(),
                    self.catalog_notes.clone(),
                    self.load_measurements.clone(),
                );
                crate::log_line(format!(
                    "action=patch-select event=open result=success count={count}"
                ));
            }
        }
    }

    /// 開いた時点で Loading だった一覧を、host app の loader 完了時に差し替える。
    ///
    /// Loading 中に Ctrl+T が押されていれば、Ready になった同じタイミングで selector を
    /// 自動で開く。overlay の開き直しを要求しないことがこの同期 API の責務。
    pub fn sync_patch_catalog(
        &mut self,
        catalog: PatchCatalogSnapshot,
        load_measurements: BTreeMap<String, PatchLoadMeasurement>,
    ) {
        if !self.open || !matches!(&self.patch_catalog, PatchCatalogSnapshot::Loading) {
            return;
        }
        let requested = self.patch_select_requested;
        let result = match &catalog {
            PatchCatalogSnapshot::Loading => return,
            PatchCatalogSnapshot::Ready(patches) => format!("ready count={}", patches.len()),
            PatchCatalogSnapshot::Error(error) => format!("error detail={error:?}"),
        };
        self.patch_catalog = catalog;
        self.load_measurements = load_measurements;
        crate::log_line(format!(
            "action=patch-catalog event=sync result={result} open_requested={requested}"
        ));
        if requested {
            self.open_patch_select();
        }
    }

    /// host app が毎 frame の loader polling を Loading 中だけに絞るための問い合わせ。
    pub fn is_waiting_for_patch_catalog(&self) -> bool {
        self.open && matches!(&self.patch_catalog, PatchCatalogSnapshot::Loading)
    }

    pub fn is_patch_select_open(&self) -> bool {
        self.patch_select.is_some()
    }

    pub(crate) fn patch_catalog_notice(&self) -> Option<&PatchCatalogNotice> {
        self.patch_catalog_notice.as_ref()
    }

    pub(super) fn handle_patch_select_key(
        &mut self,
        key: KeyEvent,
        now: Instant,
    ) -> MmlOverlayAction {
        let Some(select) = self.patch_select.as_mut() else {
            return MmlOverlayAction::Continue;
        };
        match select.handle_key(key) {
            PatchSelectAction::Continue => MmlOverlayAction::Continue,
            PatchSelectAction::Preview(patch) => MmlOverlayAction::SetPatch {
                patch: Some(patch),
                notes: self.preview_notes(now),
            },
            PatchSelectAction::Confirm(patch) => {
                // 試聴で読み込み済みの音色がそのまま残るので、ここでは積み直さない。
                self.patch_select = None;
                self.patch = Some(patch);
                MmlOverlayAction::Continue
            }
            PatchSelectAction::Cancel => self.cancel_patch_select(),
        }
    }

    fn cancel_patch_select(&mut self) -> MmlOverlayAction {
        let Some(select) = self.patch_select.take() else {
            return MmlOverlayAction::Continue;
        };
        if select.previewed() == select.original() {
            return MmlOverlayAction::Continue;
        }
        MmlOverlayAction::SetPatch {
            patch: select.original().map(str::to_string),
            notes: None,
        }
    }

    /// 音色を切り替えた直後に鳴らす音。
    ///
    /// カーソル位置に音があればそれを鳴らし直す。まだ MML が空でも音色は聴きたいので、
    /// その場合だけ試聴用の音を1つ鳴らす。
    fn preview_notes(&mut self, _now: Instant) -> Option<super::NoteRequest> {
        let notes = self.notes_at_cursor();
        self.last_notes.clone_from(&notes);
        notes
            .map(|(_, notes)| notes)
            .or_else(preview_note)
            .map(|notes| self.start_notes(&notes))
    }
}

/// MML が空のときに音色の試聴だけを目的に鳴らす 1 音。
///
/// 音高も velocity も音長も既定値のまま鳴らしたいだけなので、MML を書いて
/// 本家に解かせる。ここで組み立てると本家の既定値と二重に持つことになる。
fn preview_note() -> Option<CursorNotes> {
    notes_at_prefix(PREVIEW_MML)
}
