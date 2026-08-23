use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Modifier;

#[test]
fn keyboard_screen_shows_connecting_status_and_navigation() {
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::Keyboard;

    let screen = render_lines(&mut app, 90, 14).join("\n");

    assert!(screen.contains("[KEYBOARD] keyboard mode"));
    assert!(screen.contains("transport: SHM"));
    assert!(screen.contains("buffer: x4"));
    assert!(screen.contains("server: idle"));
    assert!(screen.contains("last send: -"));
    assert!(screen.contains("connecting..."));
    assert!(screen.contains("notes unavailable until ready"));
    assert!(!screen.contains("s:transport"));
    assert!(!screen.contains("h:transport"));
    assert!(screen.contains("Shift+H:buffer"));
    assert!(screen.contains("n:notepad"));
    assert!(screen.contains("w:DAW"));
    assert!(screen.contains("v:velocity"));
    assert!(screen.contains("m:mod(CC1)"));
    assert!(screen.contains("p:pitch bend"));
    assert!(screen.contains("t:off/repeat/arp/auto"));
    assert!(screen.contains("Note mode: off"));
    assert!(screen.contains("x:CC#"));
    assert!(screen.contains("z:CC value"));
    assert!(screen.contains("Shift+Z:CC cycle"));
    assert!(screen.contains("r:random"));
    assert!(screen.contains("Vel: 100"));
    assert!(screen.contains("Mod: OFF"));
    assert!(screen.contains("PB: -"));
    assert!(screen.contains("CC#: 1"));
}

#[test]
fn keyboard_screen_shows_count_input_guide_until_navigation() {
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::Keyboard;
    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));

    let buffer = render_buffer(&mut app, 140, 14);
    let (count_x, count_y) = find_text(&buffer, "Count: 11_");
    let (guide_x, guide_y) = find_text(&buffer, "0-9");
    assert_eq!(buffer.cell((count_x, count_y)).unwrap().fg, MONOKAI_YELLOW);
    assert!(buffer
        .cell((count_x, count_y))
        .unwrap()
        .modifier
        .contains(Modifier::BOLD));
    assert_eq!(buffer.cell((guide_x, guide_y)).unwrap().fg, MONOKAI_CYAN);
    assert!(buffer
        .cell((guide_x, guide_y))
        .unwrap()
        .modifier
        .contains(Modifier::BOLD));

    let screen = render_lines(&mut app, 140, 14).join("\n");
    assert!(screen.contains("Count: 11_"));
    assert!(screen
        .replace(' ', "")
        .contains("0-9またはh/j/k/l/Ctrl+u/Ctrl+dを押してください"));
    assert!(screen.contains("1-9:count"));
    assert!(!screen.contains("k/j/Up/Down:patch"));
    assert!(!screen.contains("s:transport"));
    assert!(!screen.contains("i:MML notes"));

    app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE));
    let screen = render_lines(&mut app, 140, 14).join("\n");
    assert!(screen.contains("1-9:count"));
    assert!(!screen.contains("Count: 11_"));
}

#[test]
fn keyboard_screen_shows_category_and_patch_panes_while_connecting() {
    let mut app = TuiApp::new_for_test(test_config());
    let patches = [
        "patches_factory/Lead/Factory Lead.fxp",
        "patches_factory/Pad/Factory Pad.fxp",
        "patches_3rdparty/vendor/Pad/Third Pad.fxp",
    ]
    .into_iter()
    .map(|patch| (patch.to_string(), patch.to_lowercase()))
    .collect();
    app.patch_load_state = std::sync::Arc::new(std::sync::Mutex::new(
        crate::tui::PatchLoadState::ready(patches),
    ));
    app.start_keyboard(Some("patches_factory/Pad/Factory Pad.fxp".to_string()));

    let screen = render_lines(&mut app, 140, 12).join("\n");

    assert!(screen.contains("Categories (2/2)"));
    assert!(screen.contains("Lead (1)"));
    assert!(screen.contains("Pad (2)"));
    assert!(screen.contains("Patches (1/2)"));
    assert!(screen.contains("patches_factory/Pad/Factory Pad.fxp"));
    assert!(screen.contains("connecting..."));
    assert!(screen.contains("k/j/Up/Down:patch -/+1"));
    assert!(screen.contains("Ctrl+u/d/PgUp/PgDn:patch -/+10"));
    assert!(screen.contains("h/l/Home/End:cat -/+1"));
    assert!(screen.contains("r:random"));
}

#[test]
fn keyboard_patch_panes_show_loading_error_and_empty_states() {
    let mut loading = TuiApp::new_for_test(test_config());
    loading.patch_load_state =
        std::sync::Arc::new(std::sync::Mutex::new(crate::tui::PatchLoadState::Loading));
    loading.active_screen = crate::screen_switch::PrimaryScreen::Keyboard;
    let screen = render_lines(&mut loading, 140, 12).join("\n");
    assert!(screen.replace(' ', "").contains("パッチを読み込み中..."));

    let mut error = TuiApp::new_for_test(test_config());
    error.patch_load_state = std::sync::Arc::new(std::sync::Mutex::new(
        crate::tui::PatchLoadState::Err("boom".to_string()),
    ));
    error.active_screen = crate::screen_switch::PrimaryScreen::Keyboard;
    let screen = render_lines(&mut error, 140, 12).join("\n");
    assert!(screen.replace(' ', "").contains("読み込み失敗:boom"));

    let mut empty = TuiApp::new_for_test(test_config());
    empty.active_screen = crate::screen_switch::PrimaryScreen::Keyboard;
    let screen = render_lines(&mut empty, 140, 12).join("\n");
    assert!(screen.replace(' ', "").contains("パッチが見つかりません"));
}
