#[cfg(not(test))]
pub(crate) fn set_text(text: String) {
    if let Ok(mut clipboard) = arboard::Clipboard::new() {
        let _ = clipboard.set_text(text);
    }
}

#[cfg(test)]
use std::cell::RefCell;

#[cfg(test)]
thread_local! {
    static TEST_CLIPBOARD: RefCell<Option<String>> = RefCell::new(None);
}

#[cfg(test)]
pub(crate) fn set_text(text: String) {
    TEST_CLIPBOARD.with(|clipboard| *clipboard.borrow_mut() = Some(text));
}

#[cfg(test)]
pub(crate) fn take_text_for_test() -> Option<String> {
    TEST_CLIPBOARD.with(|clipboard| clipboard.borrow_mut().take())
}
