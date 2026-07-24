//! OS クリップボードへの書き込み（画面横断で共有）。
//!
//! `test-support` feature を有効にすると、実クリップボードに触れずスレッドローカルへ
//! 書き込むスタブに差し替わる。app 側のテストはこの feature を dev-dependencies で
//! 有効化し、`take_text_for_test()` で書き込み内容を検証する。

#[cfg(not(feature = "test-support"))]
pub fn set_text(text: String) {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(text);
    }
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
pub fn take_text_for_test() -> Option<String> {
    TEST_CLIPBOARD.with(|clipboard| clipboard.borrow_mut().take())
}
