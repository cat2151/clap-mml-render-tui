//! 音色選択で使う修飾キーとモード切替キーの判定。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) fn is_add_preset_key(key: KeyEvent) -> bool {
    key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('a')
}

pub(super) fn is_random_jump_key(key: KeyEvent) -> bool {
    key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('r')
}

pub(super) fn is_filter_edit_trigger(key: KeyEvent) -> bool {
    key.modifiers == KeyModifiers::NONE && key.code == KeyCode::Char('/')
}

/// 音色選択中に、選択中の音色で現在行を試聴する。
pub(super) fn is_preview_key(key: KeyEvent) -> bool {
    key.modifiers == KeyModifiers::NONE && key.code == KeyCode::Char(' ')
}

/// このキーは音色選択を開く。
pub fn is_patch_select_trigger(key: KeyEvent) -> bool {
    key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('t')
}
