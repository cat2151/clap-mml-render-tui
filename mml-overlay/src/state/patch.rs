//! `Ctrl+T` の音色選択。
//!
//! 音色は入力欄のテキストと別に持つ。行が増えると行頭 JSON を全部の行へ書くことに
//! なって邪魔になるうえ、聴き比べたいのはフレーズのほうで音色は共通、という使い方が
//! 前提のため。選んだ音色は枠のタイトルにだけ出る。

use std::time::Instant;

use crossterm::event::KeyEvent;

use crate::cursor_notes::{notes_at_prefix, CursorNotes, PREVIEW_MML};
use crate::patch_select::{PatchSelect, PatchSelectAction};

use super::{MmlOverlay, MmlOverlayAction};

impl MmlOverlay<'_> {
    pub(super) fn open_patch_select(&mut self) {
        self.patch_select = PatchSelect::open(self.patches.clone(), self.patch.as_deref());
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
            PatchSelectAction::Preview(patch) => {
                let messages = self.preview_notes(now);
                MmlOverlayAction::SetPatch {
                    patch: Some(patch),
                    messages,
                }
            }
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
            messages: Vec::new(),
        }
    }

    /// 音色を切り替えた直後に鳴らす音。
    ///
    /// カーソル位置に音があればそれを鳴らし直す。まだ MML が空でも音色は聴きたいので、
    /// その場合だけ試聴用の音を1つ鳴らす。
    fn preview_notes(&mut self, now: Instant) -> Vec<[u8; 3]> {
        let notes = self.notes_at_cursor();
        self.last_notes.clone_from(&notes);
        match notes.map(|(_, notes)| notes).or_else(preview_note) {
            Some(notes) => self.start_notes(&notes, now),
            None => Vec::new(),
        }
    }
}

/// MML が空のときに音色の試聴だけを目的に鳴らす 1 音。
///
/// 音高も velocity も音長も既定値のまま鳴らしたいだけなので、MML を書いて
/// 本家に解かせる。ここで組み立てると本家の既定値と二重に持つことになる。
fn preview_note() -> Option<CursorNotes> {
    notes_at_prefix(PREVIEW_MML)
}
