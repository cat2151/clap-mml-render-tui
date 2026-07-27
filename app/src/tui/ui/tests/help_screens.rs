//! loop browser 画面のヘルプ overlay 描画（app の画面振り分けを通した確認）。

use super::*;
use crossterm::event::KeyCode;

#[test]
fn loop_browser_draws_centered_help_for_the_focused_pane() {
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::LoopBrowser;
    app.loop_browser.state.handle_key(KeyCode::Char('?'));

    let tree_screen = render_lines(&mut app, 160, 40).join("\n");
    let tree_buffer = render_buffer(&mut app, 160, 40);
    let normalized_tree = tree_screen.replace(' ', "");

    assert!(normalized_tree.contains("ヘルプ(Keybinds):LoopTree"));
    assert!(normalized_tree.contains("dirカテゴリ設定"));
    assert!(!tree_screen.contains("mix level overlay"));
    assert_help_is_centered(&tree_buffer);

    app.loop_browser.state.handle_key(KeyCode::Char('?'));
    app.loop_browser.state.handle_key(KeyCode::Tab);
    app.loop_browser.state.handle_key(KeyCode::Char('?'));

    let tracks_screen = render_lines(&mut app, 160, 40).join("\n");
    let normalized_tracks = tracks_screen.replace(' ', "");

    assert!(normalized_tracks.contains("ヘルプ(Keybinds):Tracks"));
    assert!(tracks_screen.contains("mix level overlay"));
    assert!(normalized_tracks.contains("Shift+R"));
    assert!(normalized_tracks.contains("Shift+M"));
    assert!(normalized_tracks.contains("Alt+↓/↑"));
    assert!(!normalized_tracks.contains("dirカテゴリ設定"));
    assert!(normalized_tracks.contains("[Esc/q/?]でヘルプを閉じる"));
}

/// メモリ行は overlay の先頭に出す。ヘルプが端末より長いと下が切り落とされるため。
#[test]
fn loop_browser_help_shows_the_memory_usage_at_the_top() {
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::LoopBrowser;
    app.loop_browser.state.handle_key(KeyCode::Char('?'));

    let buffer = render_buffer(&mut app, 160, 40);
    let (_, top, _, _) = help_overlay_bounds(&buffer);
    let memory_line = render_lines(&mut app, 160, 40)[usize::from(top) + 1].replace(' ', "");

    assert!(memory_line.contains("実メモリ合計"), "{memory_line}");
    assert!(memory_line.contains("OS空き"), "{memory_line}");
}

fn assert_help_is_centered(buffer: &ratatui::buffer::Buffer) {
    let (left, top, right, bottom) = help_overlay_bounds(buffer);
    let right_margin = buffer.area.width - right - 1;
    let bottom_margin = buffer.area.height - bottom - 1;

    assert!(
        left.abs_diff(right_margin) <= 1,
        "left={left} right={right_margin}"
    );
    assert!(
        top.abs_diff(bottom_margin) <= 1,
        "top={top} bottom={bottom_margin}"
    );
}
