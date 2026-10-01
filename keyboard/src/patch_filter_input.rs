//! `/` で開く、Patches pane の絞り込み入力欄。
//!
//! 打つたびに一覧を絞り直す。条件そのものは `KeyboardPatchCatalog` が持ち、ここは
//! 入力中の欄と「`Esc` で戻す先」だけを持つ。

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui_textarea::TextArea;

use cmrt_tui_core::text_input::{
    apply_key_event_to_textarea, new_single_line_textarea, textarea_value,
};

use super::{KeyboardAction, KeyboardContext, KeyboardScreen};

pub struct KeyboardPatchFilterInput<'a> {
    active: bool,
    textarea: TextArea<'a>,
    /// 入力を始める前の条件。`Esc` でここへ戻す。
    before: String,
    /// 欄の文字列が正規表現としてコンパイルできない。一覧は最後に通った条件のまま。
    invalid: bool,
}

impl Default for KeyboardPatchFilterInput<'_> {
    fn default() -> Self {
        Self {
            active: false,
            textarea: new_single_line_textarea(""),
            before: String::new(),
            invalid: false,
        }
    }
}

impl<'a> KeyboardPatchFilterInput<'a> {
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn textarea(&self) -> &TextArea<'a> {
        &self.textarea
    }

    pub fn value(&self) -> String {
        textarea_value(&self.textarea)
    }

    pub fn is_invalid(&self) -> bool {
        self.invalid
    }

    fn open(&mut self, current: &str) {
        self.textarea = new_single_line_textarea(current);
        self.before = current.to_string();
        self.invalid = false;
        self.active = true;
    }

    /// 欄を閉じる。一覧は今掛かっている条件のまま。
    pub(crate) fn close(&mut self) {
        self.active = false;
        self.invalid = false;
    }
}

impl KeyboardScreen<'_> {
    pub(crate) fn open_patch_filter(&mut self, ctx: &KeyboardContext<'_>) {
        self.sync_patch_catalog(ctx);
        self.state.patch_catalog.focus_patches();
        let current = self.state.patch_catalog.filter().to_string();
        self.patch_filter.open(&current);
    }

    pub(crate) fn handle_patch_filter_key(
        &mut self,
        key: KeyEvent,
        ctx: &KeyboardContext<'_>,
    ) -> KeyboardAction {
        if key.kind == KeyEventKind::Release {
            self.release_note_while_typing(key);
            return KeyboardAction::Continue;
        }
        let enter = key.code == KeyCode::Enter
            || (key.code == KeyCode::Char('m') && key.modifiers == KeyModifiers::CONTROL);
        if enter {
            // 不正な条件では閉じない。欄の赤枠を見て直してもらう。
            if !self.patch_filter.invalid {
                self.patch_filter.close();
            }
            return KeyboardAction::Continue;
        }
        if key.code == KeyCode::Esc {
            let before = std::mem::take(&mut self.patch_filter.before);
            self.patch_filter.close();
            self.apply_patch_filter(&before, ctx);
            return KeyboardAction::Continue;
        }
        if apply_key_event_to_textarea(&mut self.patch_filter.textarea, key) {
            let value = self.patch_filter.value();
            self.patch_filter.invalid = !self.apply_patch_filter(&value, ctx);
        }
        KeyboardAction::Continue
    }

    /// 条件を掛けて、音色が変わったら鳴らせる状態にする。コンパイルできなければ `false`。
    pub(crate) fn apply_patch_filter(
        &mut self,
        condition: &str,
        ctx: &KeyboardContext<'_>,
    ) -> bool {
        self.sync_patch_catalog(ctx);
        match self.state.patch_catalog.set_filter(condition) {
            Ok(selected) => {
                self.apply_patch_selection(selected, ctx);
                true
            }
            Err(_) => false,
        }
    }
}

#[cfg(test)]
mod tests;
