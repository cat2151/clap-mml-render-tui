//! 1 行モードの入力欄。
//!
//! 複数行モードは「フレーズを書き並べて聴き比べる画面」だが、1 行モードは
//! 「呼び出し側の 1 か所へ書き戻すための入力欄」で、`Enter` が改行ではなく確定になる。
//! 確定は [`MmlOverlayAction::Commit`] としてホストへ渡すだけで、どこへどう書くかは
//! ここでは決めない（DAW なら小節セル、Chord Chart なら stable id の section）。
//!
//! **確定キーの判定は音色選択・フレーズ履歴・演奏設定への委譲より後。**
//! それらのモーダルの `Enter` は候補の確定なので、横取りすると選べなくなる。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_textarea::TextArea;

use super::{MmlOverlay, MmlOverlayAction, MmlOverlayInputMode, SingleLineFlow};

impl MmlOverlay<'_> {
    /// いまの入力モード。ホストが `Commit` の扱いを決めるのに使う。
    pub fn input_mode(&self) -> MmlOverlayInputMode {
        self.input_mode
    }

    /// 1 行入力の完了規約。複数行モードでは参照されない。
    pub fn single_line_flow(&self) -> SingleLineFlow {
        self.single_line_flow
    }

    /// 1 行モードが食べるキーなら、その結果を返す。
    ///
    /// 呼ぶのは [`MmlOverlay::handle_key`] の、どのモーダルへの委譲よりも後。
    pub(super) fn intercept_single_line_key(&mut self, key: KeyEvent) -> Option<MmlOverlayAction> {
        if self.input_mode != MmlOverlayInputMode::SingleLine {
            return None;
        }
        match self.single_line_flow {
            SingleLineFlow::Advance if is_commit_key(key) => Some(self.commit_advance_line(false)),
            SingleLineFlow::Advance if key.code == KeyCode::Esc => {
                Some(self.commit_advance_line(true))
            }
            SingleLineFlow::Modal if is_commit_key(key) => Some(self.commit_modal_line()),
            SingleLineFlow::Modal if key.code == KeyCode::Esc => {
                self.release_context();
                Some(MmlOverlayAction::Close)
            }
            _ => None,
        }
    }

    /// Advance 1 行モードの確定。`close` なら閉じるところまでやる。
    ///
    /// 閉じない確定でも入力欄は触らない。次に何を編集するかを決めるのはホストで、
    /// ホストは [`MmlOverlay::open`] を呼び直して次の対象の内容を入れる。
    ///
    /// **打った文字列がコード表記として読めるときだけ、確定の直前にダイアログが立つ。**
    /// `Enter` でも `Esc` でも同じで、そこを素通しすると「MML のつもりで書いた
    /// コード表記が無音のセルとして残る」（発端のバグ）が確定の 2 経路のうち
    /// 片方だけ塞がれた状態になる。
    fn commit_advance_line(&mut self, close: bool) -> MmlOverlayAction {
        let line = self.current_line().to_string();
        if self.intercept_commit_for_chord_transfer(&line, close) {
            return MmlOverlayAction::Continue;
        }
        if close {
            self.release_context();
        }
        MmlOverlayAction::Commit { line, close }
    }

    /// Modal 1 行モードの確定。確定キーの種類によらず overlay も閉じる。
    fn commit_modal_line(&mut self) -> MmlOverlayAction {
        let line = self.current_line().to_string();
        self.release_context();
        MmlOverlayAction::Commit { line, close: true }
    }
}

/// 開くときの入力欄を作る。
///
/// 1 行モードでは初期テキストを入れてカーソルを末尾へ置く（既存の内容を
/// 続きから直せるように）。複数行モードは従来どおり常に空。
pub(super) fn new_textarea<'a>(
    input_mode: MmlOverlayInputMode,
    initial_text: &str,
) -> TextArea<'a> {
    match input_mode {
        MmlOverlayInputMode::MultiLine => {
            cmrt_tui_core::text_input::new_multi_line_textarea(Vec::new())
        }
        MmlOverlayInputMode::SingleLine => {
            let mut textarea =
                cmrt_tui_core::text_input::new_single_line_textarea(first_line(initial_text));
            textarea.move_cursor(ratatui_textarea::CursorMove::End);
            textarea
        }
    }
}

/// 1 行入力欄の確定キー。
///
/// 端末では `Ctrl+M` が `Enter` と同じバイトで届くことがあるが、crossterm は
/// `Ctrl+M` として渡してくることもあるので両方拾う（グローバルの 1 行入力欄の作法）。
fn is_commit_key(key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Enter => true,
        KeyCode::Char('m') => key.modifiers.contains(KeyModifiers::CONTROL),
        _ => false,
    }
}

/// 改行より前だけを取る。
///
/// 1 行入力欄へ複数行が来る経路（初期テキスト・フレーズ履歴の確定）で使う。
/// 改行を単に取り除いて連結すると、誰も書いていないフレーズが出来てしまうので
/// 先頭の 1 行だけを採る。
pub(super) fn first_line(text: &str) -> &str {
    text.lines().next().unwrap_or("")
}
