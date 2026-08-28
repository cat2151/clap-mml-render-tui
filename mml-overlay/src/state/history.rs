//! `Ctrl+O` のフレーズ履歴。
//!
//! 履歴を土台にして編集と演奏を試すための入り口。確定すると入力欄をその 1 行で
//! 上書きする（それまで書いていた内容は捨てる。オーバーレイの入力はもともと揮発）。

use crossterm::event::KeyEvent;

use crate::history_select::{HistoryPick, HistorySelect, HistorySelectAction};
use crate::line_play::LineProgram;

use super::{MmlOverlay, MmlOverlayAction, MmlOverlayInputMode, PatchChange};

impl MmlOverlay<'_> {
    pub(super) fn open_history_select(&mut self) {
        self.history_select = HistorySelect::open(self.history.clone(), self.favorites.clone());
    }

    pub(super) fn handle_history_select_key(&mut self, key: KeyEvent) -> MmlOverlayAction {
        let Some(select) = self.history_select.as_mut() else {
            return MmlOverlayAction::Continue;
        };
        match select.handle_key(key) {
            HistorySelectAction::Continue => MmlOverlayAction::Continue,
            HistorySelectAction::Preview(pick) => self.play_pick(&pick),
            HistorySelectAction::Confirm(pick) => {
                self.history_select = None;
                self.replace_text(&pick.mml);
                if let Some(patch) = &pick.patch {
                    self.patch = Some(patch.clone());
                }
                // 試聴で鳴っているものと同じなので、ここでは積み直さない。
                MmlOverlayAction::Continue
            }
            HistorySelectAction::Cancel => self.cancel_history_select(),
        }
    }

    fn cancel_history_select(&mut self) -> MmlOverlayAction {
        let Some(select) = self.history_select.take() else {
            return MmlOverlayAction::Continue;
        };
        if !select.previewed() {
            return MmlOverlayAction::Continue;
        }
        // 試聴で差し替えた音色を戻し、鳴っている演奏も止める。
        MmlOverlayAction::PlayLine {
            patch: PatchChange::Switch(self.patch.clone()),
            program: LineProgram::silent(),
        }
    }

    /// 履歴の 1 行を、その行に書かれていた音色で鳴らす。
    ///
    /// 音色を持たない項目（notepad で音色を指定せずに書いた行）は、いまの音色のまま
    /// 鳴らす。既定音色へ戻してしまうと、選んでいる音色でフレーズを試せなくなる。
    fn play_pick(&mut self, pick: &HistoryPick) -> MmlOverlayAction {
        let (status, performance) = crate::line_play::line_events(&pick.mml);
        self.line_status = status;
        let patch = match &pick.patch {
            Some(patch) => PatchChange::Switch(Some(patch.clone())),
            None => PatchChange::Keep,
        };
        MmlOverlayAction::PlayLine {
            patch,
            program: self.play_settings.program(performance),
        }
    }

    /// 入力欄をこの 1 行で置き換える。
    ///
    /// 1 行モードでは 1 行入力欄として作り直す（複数行の履歴が来ても改行を
    /// 持ち込まない）。ここで複数行の入力欄に戻すと `Enter` が改行に戻ってしまう。
    fn replace_text(&mut self, mml: &str) {
        self.textarea = match self.input_mode() {
            MmlOverlayInputMode::MultiLine => {
                cmrt_tui_core::text_input::new_multi_line_textarea(vec![mml.to_string()])
            }
            MmlOverlayInputMode::SingleLine => cmrt_tui_core::text_input::new_single_line_textarea(
                super::single_line::first_line(mml),
            ),
        };
        self.textarea.move_cursor(ratatui_textarea::CursorMove::End);
        self.forget_cursor_unit();
    }
}
