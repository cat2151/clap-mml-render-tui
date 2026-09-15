//! 音色選択で使う修飾キーとモード切替キーの判定。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub(super) fn is_add_preset_key(key: KeyEvent) -> bool {
    is_plain_char(key, 'a')
}

pub(super) fn is_random_jump_key(key: KeyEvent) -> bool {
    is_plain_char(key, 'r')
}

/// selector 内から演奏設定を開閉する。
pub(crate) fn is_patch_select_play_settings_trigger(key: KeyEvent) -> bool {
    is_plain_char(key, 's')
}

fn is_plain_char(key: KeyEvent, character: char) -> bool {
    key.modifiers == KeyModifiers::NONE && key.code == KeyCode::Char(character)
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
