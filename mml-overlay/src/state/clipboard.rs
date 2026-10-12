//! 入力欄の `Ctrl+C` / `Ctrl+X` / `Ctrl+V`。OS クリップボードと読み書きする
//! （DAW の INSERT と同じ [`cmrt_tui_core::clipboard::handle_textarea_key`]）。

use crossterm::event::KeyEvent;

use super::{MmlOverlay, MmlOverlayInputMode};

impl MmlOverlay<'_> {
    /// 打鍵を入力欄へ渡す。`Ctrl+C` / `Ctrl+X` / `Ctrl+V` はクリップボードとの読み書きにする。
    pub(super) fn input_textarea(&mut self, key: KeyEvent) {
        let single_line = self.input_mode == MmlOverlayInputMode::SingleLine;
        if !cmrt_tui_core::clipboard::handle_textarea_key(&mut self.textarea, key, single_line) {
            self.textarea.input(key);
        }
    }
}
