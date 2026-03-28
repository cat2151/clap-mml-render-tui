//! UI ユーティリティ（TUI / DAW 共通）

use ratatui::layout::{Constraint, Direction, Layout, Rect};
use tui_textarea::TextArea;

/// 指定した割合で中央に配置した矩形を返す。ポップアップ表示に利用する。
pub(crate) fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let px = percent_x.min(100);
    let py = percent_y.min(100);
    let v = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - py) / 2),
            Constraint::Percentage(py),
            Constraint::Percentage((100 - py) / 2),
        ])
        .split(area);
    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - px) / 2),
            Constraint::Percentage(px),
            Constraint::Percentage((100 - px) / 2),
        ])
        .split(v[1])[1]
}

pub(crate) fn copy_textarea_selection(textarea: &mut TextArea<'_>) {
    textarea.copy();
    let yanked = textarea.yank_text();
    write_clipboard_text(&yanked);
}

pub(crate) fn cut_textarea_selection(textarea: &mut TextArea<'_>) {
    textarea.cut();
    let yanked = textarea.yank_text();
    write_clipboard_text(&yanked);
}

pub(crate) fn paste_textarea_selection(textarea: &mut TextArea<'_>) {
    let Some(text) = read_clipboard_text() else {
        return;
    };
    textarea.set_yank_text(&text);
    textarea.paste();
}

fn write_clipboard_text(text: &str) {
    #[cfg(test)]
    {
        *test_clipboard_cell().lock().unwrap() = Some(text.to_string());
    }

    #[cfg(not(test))]
    {
        if let Ok(mut clipboard) = arboard::Clipboard::new() {
            let _ = clipboard.set_text(text.to_string());
        }
    }
}

fn read_clipboard_text() -> Option<String> {
    #[cfg(test)]
    {
        return test_clipboard_cell().lock().unwrap().clone();
    }

    #[cfg(not(test))]
    {
        let mut clipboard = arboard::Clipboard::new().ok()?;
        clipboard.get_text().ok()
    }
}

#[cfg(test)]
fn test_clipboard_cell() -> &'static std::sync::Mutex<Option<String>> {
    use std::sync::{Mutex, OnceLock};

    static TEST_CLIPBOARD: OnceLock<Mutex<Option<String>>> = OnceLock::new();

    TEST_CLIPBOARD.get_or_init(|| Mutex::new(None))
}

#[cfg(test)]
fn test_clipboard_lock() -> &'static std::sync::Mutex<()> {
    use std::sync::{Mutex, OnceLock};

    static TEST_CLIPBOARD_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    TEST_CLIPBOARD_LOCK.get_or_init(|| Mutex::new(()))
}

#[cfg(test)]
pub(crate) struct TestClipboardGuard {
    _guard: std::sync::MutexGuard<'static, ()>,
}

#[cfg(test)]
impl TestClipboardGuard {
    pub(crate) fn new(initial: Option<&str>) -> Self {
        let guard = test_clipboard_lock().lock().unwrap();
        *test_clipboard_cell().lock().unwrap() = initial.map(str::to_string);
        Self { _guard: guard }
    }

    pub(crate) fn text(&self) -> Option<String> {
        test_clipboard_cell().lock().unwrap().clone()
    }
}
