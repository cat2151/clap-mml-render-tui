//! notepad のカーソル行を、共有 MML overlay の 1 行モードで編集する配線。
//!
//! 入力欄には行頭 JSON を除いた MML だけを入れ、音色は行頭 JSON から overlay の音色へ、
//! effect chain は sender の音色準備へ渡す。確定時の書き戻しは notepad 側
//! （`NotepadScreen::commit_line_edit`）で、行頭 JSON は元の文字列のまま残る。
//!
//! `SingleLineFlow::Advance` で開く。`Enter` は確定して下へ空行を足し、overlay を
//! 開いたままその行へ移る。`Esc` は確定して閉じる。どちらの確定でも notepad の
//! 従来どおり offline render で鳴らすので、`Enter` の開き直しでは notepad の演奏を止めない。

use cmrt_mml_overlay::patch_json::{effect_chain_json, patch_name, strip_patch_json};
use cmrt_mml_overlay::LivePatch;
use cmrt_notepad::NotepadScreen;

use super::super::mml_overlay::{MmlOverlayContext, MmlOverlayInputMode, SingleLineFlow};
use super::super::{MmlOverlayOwner, TuiApp};

impl TuiApp<'_> {
    /// notepad の `i` / `o` / `O` の行き先。カーソル行を overlay で開く。
    pub(in crate::tui) fn open_notepad_line_overlay(&mut self) {
        let context = self.notepad_line_overlay_context();
        self.open_owned_mml_overlay(MmlOverlayOwner::Notepad, context);
    }

    fn notepad_line_overlay_context(&self) -> MmlOverlayContext {
        let line = self.notepad.current_line();
        let (history, favorites) = self
            .notepad
            .phrase_history_for_patch(patch_name(line).as_deref());
        MmlOverlayContext {
            input_mode: MmlOverlayInputMode::SingleLine,
            single_line_flow: SingleLineFlow::Advance,
            initial_text: strip_patch_json(line).to_string(),
            history,
            favorites,
            ..self.mml_overlay_context()
        }
    }

    /// `Enter` の確定。書き戻して下へ空行を足し、overlay をその行で開き直す。
    pub(super) fn advance_notepad_line_overlay(&mut self, body: &str) {
        self.commit_notepad_line(body);
        self.notepad.insert_line_below_for_edit();
        let context = self.notepad_line_overlay_context();
        self.attach_owned_mml_overlay(MmlOverlayOwner::Notepad, context);
    }

    /// 入力欄の MML を行へ書き戻す。音色は overlay で選び直したときだけ渡す。
    pub(super) fn commit_notepad_line(&mut self, body: &str) {
        let changed_patch = self
            .mml_overlay
            .patch()
            .filter(|patch| line_patch(&self.notepad).as_deref() != Some(*patch))
            .map(str::to_owned);
        self.notepad
            .commit_line_edit(body, changed_patch.as_deref());
    }
}

/// カーソル行の行頭 JSON の音色。
pub(super) fn line_patch(notepad: &NotepadScreen<'_>) -> Option<String> {
    patch_name(notepad.current_line())
}

/// カーソル行の effect chain を載せた音色。行頭 JSON が読めなければ chain 無し。
pub(super) fn live_patch(notepad: &NotepadScreen<'_>, patch: Option<&str>) -> LivePatch {
    let chain = effect_chain_json(notepad.current_line()).unwrap_or_default();
    LivePatch::with_effect_chain(patch, &chain)
}
