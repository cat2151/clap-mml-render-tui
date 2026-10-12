//! 入力欄の `Ctrl+C` / `Ctrl+X` / `Ctrl+V`。OS クリップボードと読み書きする。

use super::*;

use crate::state::MmlOverlayInputMode;

fn single_line(initial_text: &str) -> MmlOverlay<'static> {
    let mut overlay = MmlOverlay::default();
    overlay.open(MmlOverlayContext {
        input_mode: MmlOverlayInputMode::SingleLine,
        initial_text: initial_text.to_string(),
        ..MmlOverlayContext::default()
    });
    overlay
}

/// カーソル（行末）から行頭までを選ぶ。
fn select_to_head(overlay: &mut MmlOverlay<'_>) {
    overlay.handle_key(
        KeyEvent::new(KeyCode::Home, KeyModifiers::SHIFT),
        Instant::now(),
    );
}

#[test]
fn ctrl_c_copies_the_selection_to_the_os_clipboard() {
    let mut overlay = single_line("cde");
    let _ = cmrt_tui_core::clipboard::take_text_for_test();
    select_to_head(&mut overlay);

    overlay.handle_key(ctrl(KeyCode::Char('c')), Instant::now());

    assert_eq!(overlay.value(), "cde");
    assert_eq!(
        cmrt_tui_core::clipboard::take_text_for_test(),
        Some("cde".to_string())
    );
}

#[test]
fn ctrl_x_cuts_the_selection_to_the_os_clipboard() {
    let mut overlay = single_line("cde");
    let _ = cmrt_tui_core::clipboard::take_text_for_test();
    select_to_head(&mut overlay);

    overlay.handle_key(ctrl(KeyCode::Char('x')), Instant::now());

    assert_eq!(overlay.value(), "");
    assert_eq!(
        cmrt_tui_core::clipboard::take_text_for_test(),
        Some("cde".to_string())
    );
}

/// 他のアプリでコピーした文字列も貼れる。
#[test]
fn ctrl_v_pastes_the_os_clipboard_at_the_cursor() {
    let mut overlay = single_line("o4");
    cmrt_tui_core::clipboard::set_text("cde".to_string());

    overlay.handle_key(ctrl(KeyCode::Char('v')), Instant::now());

    assert_eq!(overlay.value(), "o4cde");
    let _ = cmrt_tui_core::clipboard::take_text_for_test();
}

#[test]
fn ctrl_v_replaces_the_selection() {
    let mut overlay = single_line("cde");
    cmrt_tui_core::clipboard::set_text("efg".to_string());
    select_to_head(&mut overlay);

    overlay.handle_key(ctrl(KeyCode::Char('v')), Instant::now());

    assert_eq!(overlay.value(), "efg");
    let _ = cmrt_tui_core::clipboard::take_text_for_test();
}

/// 1 行モードは改行で確定する入力欄なので、複数行は先頭の 1 行だけを貼る。
#[test]
fn single_line_pastes_only_the_first_line() {
    let mut overlay = single_line("");
    cmrt_tui_core::clipboard::set_text("cde\nefg".to_string());

    overlay.handle_key(ctrl(KeyCode::Char('v')), Instant::now());

    assert_eq!(overlay.value(), "cde");
    let _ = cmrt_tui_core::clipboard::take_text_for_test();
}

#[test]
fn multi_line_pastes_every_line() {
    let mut overlay = opened();
    cmrt_tui_core::clipboard::set_text("cde\nefg".to_string());

    overlay.handle_key(ctrl(KeyCode::Char('v')), Instant::now());

    assert_eq!(overlay.value(), "cde\nefg");
    let _ = cmrt_tui_core::clipboard::take_text_for_test();
}

/// クリップボードが空なら入力欄は変わらない。
#[test]
fn ctrl_v_with_an_empty_clipboard_changes_nothing() {
    let mut overlay = single_line("cde");
    let _ = cmrt_tui_core::clipboard::take_text_for_test();

    overlay.handle_key(ctrl(KeyCode::Char('v')), Instant::now());

    assert_eq!(overlay.value(), "cde");
}
