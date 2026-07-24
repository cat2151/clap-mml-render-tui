//! カーソル形状の画面横断ディスパッチ。
//!
//! notepad 内部のサブモード判定は `cmrt-notepad` 側のテストが担当し、
//! ここでは「どの画面を表示しているか」で切り替わることだけを確認する。

use super::*;

#[test]
fn textarea_cursor_follows_the_active_screen() {
    let mut app = TuiApp::new_for_test(test_config());
    assert!(!app.uses_textarea_cursor());

    // notepad の INSERT では textarea カーソルになる（判定は notepad crate へ委譲）。
    app.notepad.mode = Mode::Insert;
    assert!(app.uses_textarea_cursor());

    // loop browser は textarea を持たないので、notepad のサブモードに関わらず false。
    app.active_screen = crate::screen_switch::PrimaryScreen::LoopBrowser;
    assert!(!app.uses_textarea_cursor());

    // keyboard は MML 入力中だけ textarea カーソルになる。
    app.active_screen = crate::screen_switch::PrimaryScreen::Keyboard;
    assert!(!app.uses_textarea_cursor());
}
