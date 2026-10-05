//! 移動は repeat を許可し、セル切替は Press の 1 回だけ受け付ける。

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::{DrumSequencerScreen, DRUM_STEPS};

impl DrumSequencerScreen {
    /// matrix のキーを処理したかを返す。selector の入力は host が先に消費する。
    /// kit 未選択・不明・空一覧では、入力を変更しない。
    pub fn handle_key_event(&mut self, key: KeyEvent) -> bool {
        if key.kind == KeyEventKind::Release
            || key.modifiers.intersects(
                KeyModifiers::CONTROL
                    | KeyModifiers::ALT
                    | KeyModifiers::SUPER
                    | KeyModifiers::HYPER
                    | KeyModifiers::META,
            )
        {
            return false;
        }
        let handled = matches!(
            key.code,
            KeyCode::Char('h' | 'j' | 'k' | 'l' | ' ') | KeyCode::Enter
        );
        if !handled || self.notes().is_empty() {
            return handled;
        }
        match key.code {
            KeyCode::Char('h') => self.cursor_step = self.cursor_step.saturating_sub(1),
            KeyCode::Char('l') => self.cursor_step = (self.cursor_step + 1).min(DRUM_STEPS - 1),
            KeyCode::Char('k') => self.cursor_row = self.cursor_row.saturating_sub(1),
            KeyCode::Char('j') => {
                self.cursor_row = (self.cursor_row + 1).min(self.notes().len() - 1);
            }
            KeyCode::Char(' ') | KeyCode::Enter if key.kind == KeyEventKind::Press => {
                self.toggle_cursor();
            }
            _ => {}
        }
        handled
    }
}

#[cfg(test)]
mod tests;
