//! chord のヒントと、確定ダイアログの判定。
//!
//! ヒントを立てる条件は **`chord2mml_core::convert` が Ok を返すこと**だけ
//! （[`cmrt_chord::parses_as_chord`] 経由）。degree か chord name かは見分けない。
//! 見分けても呼び出し側の動きが変わらないうえ、区別を仕様に持ち込むと
//! 「どちらの綴りが chord 扱いか」を覚える必要が出るため。
//!
//! **移送先を持たない画面（notepad / keyboard / grid）では全部無効。**
//! 判定そのものを走らせないので、複数行モードの挙動は 1 ビットも変わらない。

use crossterm::event::KeyEvent;

use crate::chord_transfer::{ChordTransferAction, ChordTransferChoice, ChordTransferConfirm};

use super::{MmlOverlay, MmlOverlayAction};

impl MmlOverlay<'_> {
    /// いまの 1 行がコード表記として読めるか（ヒントの表示用）。
    ///
    /// 移送先が無い画面では常に `false`。
    pub fn chord_hint(&self) -> bool {
        self.chord_hint
    }

    /// 確定ダイアログが開いているか（描画用）。
    pub(crate) fn chord_transfer_confirm(&self) -> Option<&ChordTransferConfirm> {
        self.chord_transfer_confirm.as_ref()
    }

    /// 入力欄の中身から chord ヒントを作り直す。
    pub(super) fn refresh_chord_hint(&mut self) {
        self.chord_hint =
            self.chord_row_transfer && cmrt_chord::parses_as_chord(self.current_line());
    }

    /// 確定が chord 行への移送になりうるなら、ダイアログを開いて `true` を返す。
    ///
    /// 呼ぶのは 1 行モードの確定判定から。ヒントが立っていないときは何もしないので、
    /// 従来の確定はそのまま通る。
    pub(super) fn intercept_commit_for_chord_transfer(&mut self, line: &str, close: bool) -> bool {
        if !self.chord_hint {
            return false;
        }
        crate::log_line(format!(
            "action=chord-transfer event=confirm-open line={line:?} close={close}"
        ));
        self.chord_transfer_confirm = Some(ChordTransferConfirm::open(line.to_string(), close));
        true
    }

    /// ダイアログが開いている間の打鍵。
    pub(super) fn handle_chord_transfer_key(&mut self, key: KeyEvent) -> MmlOverlayAction {
        let Some(confirm) = self.chord_transfer_confirm.as_mut() else {
            return MmlOverlayAction::Continue;
        };
        match confirm.handle_key(key) {
            ChordTransferAction::Continue => MmlOverlayAction::Continue,
            ChordTransferAction::Cancel => self.close_chord_transfer_confirm("cancel"),
            ChordTransferAction::Confirm(ChordTransferChoice::Transfer) => self.transfer_line(),
            ChordTransferAction::Confirm(ChordTransferChoice::KeepAsMml) => self.keep_as_mml(),
        }
    }

    /// chord 行へ移す。overlay はここで閉じる（カーソルが chord 行へ移るため）。
    fn transfer_line(&mut self) -> MmlOverlayAction {
        let Some(confirm) = self.chord_transfer_confirm.take() else {
            return MmlOverlayAction::Continue;
        };
        let line = confirm.line().to_string();
        crate::log_line(format!(
            "action=chord-transfer event=confirm-close result=transfer line={line:?}"
        ));
        self.release_context();
        MmlOverlayAction::TransferToChordRow { line }
    }

    /// ダイアログが無かった場合と同じ確定を返す。
    fn keep_as_mml(&mut self) -> MmlOverlayAction {
        let Some(confirm) = self.chord_transfer_confirm.take() else {
            return MmlOverlayAction::Continue;
        };
        let line = confirm.line().to_string();
        let close = confirm.close();
        crate::log_line(format!(
            "action=chord-transfer event=confirm-close result=keep-as-mml line={line:?}"
        ));
        if close {
            self.release_context();
        }
        MmlOverlayAction::Commit { line, close }
    }

    /// 確定そのものを取り消す。入力欄へ戻るだけで、どこにも書かない。
    fn close_chord_transfer_confirm(&mut self, result: &str) -> MmlOverlayAction {
        self.chord_transfer_confirm = None;
        crate::log_line(format!(
            "action=chord-transfer event=confirm-close result={result}"
        ));
        MmlOverlayAction::Continue
    }
}
