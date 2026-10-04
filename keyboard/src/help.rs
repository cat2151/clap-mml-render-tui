//! Keyboard の help の開閉と、表示中の入力の消費。

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::{KeyboardAction, KeyboardScreen};

fn is_help_key(key: KeyEvent) -> bool {
    key.kind == KeyEventKind::Press
        && key.code == KeyCode::Char('?')
        && matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT)
}

impl KeyboardScreen<'_> {
    pub fn help_open(&self) -> bool {
        self.help_open
    }

    /// 入力欄・選択 overlay が使わなかったキーだけを受け取る。
    pub(crate) fn handle_help_key(&mut self, key: KeyEvent) -> Option<KeyboardAction> {
        if self.help_open {
            match key.kind {
                KeyEventKind::Release => self.release_note_while_typing(key),
                KeyEventKind::Press if is_help_key(key) || key.code == KeyCode::Esc => {
                    self.help_open = false;
                }
                _ => {}
            }
            return Some(KeyboardAction::Continue);
        }
        if !is_help_key(key) {
            return None;
        }
        self.state.navigation_count.clear();
        self.effect.clear_pending_delete();
        self.help_open = true;
        Some(KeyboardAction::Continue)
    }
}

#[cfg(test)]
mod tests;
