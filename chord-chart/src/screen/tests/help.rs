//! ヘルプ overlay の開閉と、他のキーを食う範囲。共有ランタイムへ渡すキーもここ。

use super::*;

#[test]
fn question_mark_toggles_the_help_and_escape_closes_it() {
    let mut screen = three_section_screen();

    // `?` は端末によって SHIFT 付きで来る。どちらでも開くこと。
    screen.handle_key_event(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::SHIFT));
    assert!(screen.help_open);

    screen.handle_key_event(key(KeyCode::Char('?')));
    assert!(!screen.help_open);

    screen.handle_key_event(key(KeyCode::Char('?')));
    screen.handle_key_event(key(KeyCode::Esc));
    assert!(!screen.help_open);
}

/// ヘルプを閉じたときに裏の画面が動いていると、何が起きたのか分からなくなる。
#[test]
fn the_help_overlay_swallows_every_other_key() {
    let mut screen = three_section_screen();
    screen.handle_key_event(key(KeyCode::Char('?')));

    screen.handle_key_event(key(KeyCode::Char('j')));
    screen.handle_key_event(key(KeyCode::Char('l')));
    screen.handle_key_event(key(KeyCode::PageDown));
    // ヘルプを読んでいる途中の `q` でアプリが落ちない（閉じるのは `?` / `Esc`）。
    assert_eq!(
        screen.handle_key_event(key(KeyCode::Char('q'))),
        ChordChartAction::Continue
    );

    assert!(screen.help_open);
    assert_eq!(screen.section_cursor, 0);
    assert_eq!(screen.focus, Pane::Sections);
}

/// `Ctrl+G`（画面切替）は共有ランタイムのもの。取りこぼしても画面が動かないこと。
/// ALT はこの画面が `Alt+↑` `Alt+↓` にだけ使うので、それ以外の ALT 付きも同じ扱い。
#[test]
fn control_and_alt_combinations_are_left_to_the_shared_runtime() {
    let mut screen = three_section_screen();

    screen.handle_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::CONTROL));
    screen.handle_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::ALT));
    screen.handle_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::ALT));

    assert_eq!(screen.section_cursor, 0);
    assert_eq!(screen.focus, Pane::Sections);
}
