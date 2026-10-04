use super::*;
use cmrt_core::EffectPlugins;
use cmrt_tui_core::buffer_test::help_overlay_bounds;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::effect_pane::tests::{context, patch_load, screen_with};

fn render(screen: &mut KeyboardScreen<'_>) -> Terminal<TestBackend> {
    let mut terminal = Terminal::new(TestBackend::new(90, 34)).unwrap();
    terminal
        .draw(|f| {
            draw(
                screen,
                &KeyboardConnectionStatus::default(),
                Instant::now(),
                f,
            )
        })
        .unwrap();
    terminal
}

#[test]
fn the_normal_footer_keeps_the_help_entry_visible_for_both_pane_modes() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen_with(EffectPlugins::none(), Vec::new(), &ctx);

    for focus in [PatchPaneFocus::Patches, PatchPaneFocus::Effect] {
        assert_eq!(screen.state.patch_catalog.focus(), focus);
        let terminal = render(&mut screen);
        let text = buffer_to_string(&terminal);
        assert!(!text.replace(' ', "").contains("ヘルプ(Keybinds)"));
        assert!(text
            .lines()
            .rev()
            .take(3)
            .any(|line| line.contains("?:help")));
        screen.state.patch_catalog.move_focus(1);
    }
}

#[test]
fn question_mark_draws_keybinds_and_escape_returns_to_the_normal_screen() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen_with(EffectPlugins::none(), Vec::new(), &ctx);
    screen.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE), &ctx);

    let terminal = render(&mut screen);
    let text = buffer_to_string(&terminal);
    let compact = text.replace(' ', "");
    assert!(compact.contains("Keyboardヘルプ(Keybinds)"), "{text}");
    assert!(text.contains("Esc/?:close"), "{text}");
    assert!(
        compact.contains("音符を演奏(Effectfocusではcefg)"),
        "{text}"
    );
    assert!(
        compact.contains("数字+h/j/k/l・Ctrl+U/D回数を指定して移動"),
        "{text}"
    );
    assert!(
        compact.contains("Effectfocusの操作(共通操作より優先)"),
        "{text}"
    );
    assert!(compact.contains("ddEffectを削除"), "{text}");
    assert!(
        compact.contains("Alt+↑/↓Effectの順序を上/下へ移動"),
        "{text}"
    );
    assert!(
        compact.contains("?/Esc/qhelp開閉/helpを閉じる/通常状態で終了"),
        "{text}"
    );
    let (left, top, right, bottom) = help_overlay_bounds(terminal.backend().buffer());
    assert_eq!(left, 89 - right);
    assert!(top.abs_diff(33 - bottom) <= 1);

    screen.handle_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE), &ctx);
    let closed = buffer_to_string(&render(&mut screen));
    assert!(!closed.replace(' ', "").contains("ヘルプ(Keybinds)"));
    assert!(closed.contains("?:help"));
}

#[test]
fn help_is_drawn_over_the_daily_sound_check_overlay() {
    let load = patch_load();
    let ctx = context(&load);
    let mut screen = screen_with(EffectPlugins::none(), Vec::new(), &ctx);
    let now = Instant::now();
    screen.note_guide.tick(now, true, "2026-10-04");
    screen
        .note_guide
        .tick(now + std::time::Duration::from_secs(1), true, "2026-10-04");
    assert_eq!(
        screen.note_guide.presentation(),
        KeyboardNoteGuidePresentation::Overlay
    );
    assert!(buffer_to_string(&render(&mut screen))
        .replace(' ', "")
        .contains("音出し確認"));

    screen.handle_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE), &ctx);
    let text = buffer_to_string(&render(&mut screen));
    let compact = text.replace(' ', "");
    assert!(compact.contains("ヘルプ(Keybinds)"), "{text}");
    assert!(
        compact.contains("x/zCC番号/CC値を入力(Enter確定/Esc取消)"),
        "{text}"
    );
    assert!(!compact.contains("音出し確認"), "{text}");
    assert_eq!(
        screen.note_guide.presentation(),
        KeyboardNoteGuidePresentation::Overlay
    );
}
