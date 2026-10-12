//! OS クリップボードの読み書き（画面横断で共有）。
//!
//! `test-support` feature を有効にすると、実クリップボードに触れずスレッドローカルを
//! 読み書きするスタブに差し替わる。app 側のテストはこの feature を dev-dependencies で
//! 有効化し、`take_text_for_test()` で書き込み内容を検証する。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui_textarea::TextArea;

/// 入力欄の `Ctrl+C` / `Ctrl+X` / `Ctrl+V` を OS クリップボードと読み書きする。処理したら true。
///
/// `single_line` の入力欄へ複数行を貼るときは先頭の 1 行だけを貼る。改行で確定する
/// 入力欄なので、改行を残すと 1 行に収まらず、連結すると誰も書いていない文字列になる。
pub fn handle_textarea_key(textarea: &mut TextArea<'_>, key: KeyEvent, single_line: bool) -> bool {
    if !key.modifiers.contains(KeyModifiers::CONTROL) {
        return false;
    }
    match key.code {
        KeyCode::Char('c') => {
            textarea.copy();
            set_text(textarea.yank_text().to_string());
        }
        KeyCode::Char('x') => {
            textarea.cut();
            set_text(textarea.yank_text().to_string());
        }
        KeyCode::Char('v') => {
            if let Some(text) = get_text() {
                let text = if single_line {
                    text.lines().next().unwrap_or("")
                } else {
                    &text
                };
                textarea.insert_str(text);
            }
        }
        _ => return false,
    }
    true
}

#[cfg(not(feature = "test-support"))]
pub fn set_text(text: String) {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(text);
    }
}

/// クリップボードの文字列。読めない・文字列でないときは `None`。
#[cfg(not(feature = "test-support"))]
pub fn get_text() -> Option<String> {
    arboard::Clipboard::new().ok()?.get_text().ok()
}

#[cfg(feature = "test-support")]
use std::cell::RefCell;

#[cfg(feature = "test-support")]
thread_local! {
    static TEST_CLIPBOARD: RefCell<Option<String>> = const { RefCell::new(None) };
}

#[cfg(feature = "test-support")]
pub fn set_text(text: String) {
    TEST_CLIPBOARD.with(|clipboard| *clipboard.borrow_mut() = Some(text));
}

#[cfg(feature = "test-support")]
pub fn get_text() -> Option<String> {
    TEST_CLIPBOARD.with(|clipboard| clipboard.borrow().clone())
}

#[cfg(feature = "test-support")]
pub fn take_text_for_test() -> Option<String> {
    TEST_CLIPBOARD.with(|clipboard| clipboard.borrow_mut().take())
}
