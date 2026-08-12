use super::*;

#[test]
fn the_keybind_line_is_always_visible() {
    let screen = GridSequencerScreen::new(None);

    let rendered = render(&screen);

    assert!(rendered.contains("mouse:edit"), "{rendered}");
    assert!(rendered.contains("a:rnd"), "{rendered}");
    assert!(rendered.contains("x:clear"), "{rendered}");
    assert!(rendered.contains("r/R:random"), "{rendered}");
    assert!(rendered.contains("t:tracks"), "{rendered}");
    assert!(rendered.contains("Ctrl+G:screen"), "{rendered}");
    assert!(rendered.contains("q:quit"), "{rendered}");
}

/// 左 pane にキーバインド、右 pane に挙動の説明。混ぜないこと自体が仕様。
#[test]
fn the_help_overlay_splits_keybinds_from_the_feature_notes() {
    let mut screen = GridSequencerScreen::new(None);
    screen.help_open = true;

    let terminal = terminal_for(&screen);
    let buffer = terminal.backend().buffer();

    // 全角文字はセル2つぶんを占めるため、空白を無視して探す。
    find_text_ignoring_spaces(buffer, "ヘルプ(Keybinds)");
    find_text_ignoring_spaces(buffer, "画面切替メニュー");
    find_text_ignoring_spaces(buffer, "Ctrl+G");
    find_text_ignoring_spaces(buffer, "#=Attack-=Tie.=Rest");
    let rendered = buffer_to_string(&terminal);
    assert!(
        rendered
            .replace(' ', "")
            .contains("触った項目だけが自動でOFF"),
        "{rendered}"
    );
    // 左 pane の先頭（Ctrl+G）と右 pane の先頭（メモリ行）が同じ行に並ぶ＝2 pane。
    assert!(
        rendered.lines().any(|line| {
            let line = line.replace(' ', "");
            line.contains("Ctrl+G") && line.contains("実メモリ合計")
        }),
        "{rendered}"
    );
}

/// メモリ行は overlay の先頭に出す。ヘルプが端末より長いと下が切り落とされるため。
#[test]
fn the_help_overlay_shows_the_memory_usage_at_the_top() {
    let mut screen = GridSequencerScreen::new(None);
    screen.help_open = true;

    let terminal = terminal_for(&screen);
    let buffer = terminal.backend().buffer();
    let (_, top, _, _) = cmrt_tui_core::buffer_test::help_overlay_bounds(buffer);

    let memory_line = buffer_to_string(&terminal)
        .lines()
        .nth(usize::from(top) + 1)
        .unwrap()
        .replace(' ', "");

    assert!(memory_line.contains("実メモリ合計"), "{memory_line}");
    assert!(memory_line.contains("OS空き"), "{memory_line}");
}
