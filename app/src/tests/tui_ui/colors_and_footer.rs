use super::*;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::Modifier;

#[test]
fn normal_screen_uses_monokai_background_and_border_color() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string()];

    let buffer = render_buffer(&mut app, 80, 8);

    assert_eq!(buffer.cell((0, 0)).unwrap().fg, MONOKAI_CYAN);
    assert_eq!(buffer.cell((0, 0)).unwrap().bg, MONOKAI_BG);
    assert_eq!(buffer.cell((4, 4)).unwrap().fg, MONOKAI_CYAN);
    assert_eq!(buffer.cell((4, 4)).unwrap().bg, MONOKAI_BG);
}

#[test]
fn normal_screen_cursor_uses_contrast_background_without_blink() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string()];

    let buffer = render_buffer(&mut app, 80, 8);
    let (x, y) = find_text(&buffer, "abc");
    let cell = buffer.cell((x, y)).unwrap();

    assert_eq!(cell.fg, MONOKAI_FG);
    assert_eq!(cell.bg, cursor_highlight_bg(MONOKAI_FG));
    assert!(!cell
        .modifier
        .contains(ratatui::style::Modifier::RAPID_BLINK));
}

#[test]
fn insert_and_filter_modes_use_terminal_bar_cursor() {
    let mut app = TuiApp::new_for_test(test_config());

    assert!(!app.uses_textarea_cursor());

    app.mode = Mode::Insert;
    assert!(app.uses_textarea_cursor());

    app.mode = Mode::PatchSelect;
    app.patch_select_filter_active = true;
    assert!(app.uses_textarea_cursor());

    app.patch_select_filter_active = false;
    app.mode = Mode::NotepadHistory;
    app.notepad_filter_active = true;
    assert!(app.uses_textarea_cursor());

    app.notepad_filter_active = false;
    app.mode = Mode::PatchPhrase;
    app.patch_phrase_filter_active = true;
    assert!(app.uses_textarea_cursor());
}

#[test]
fn help_screen_uses_light_gray_escape_hint_on_monokai_background() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::Help;

    let buffer = render_buffer(&mut app, 80, 60);
    let (x, y) = find_text(&buffer, "[ESC]");

    assert_eq!(buffer.cell((x, y)).unwrap().fg, MONOKAI_GRAY);
    assert_eq!(buffer.cell((x, y)).unwrap().bg, MONOKAI_BG);
}

#[test]
fn status_color_uses_monokai_palette() {
    assert_eq!(status_color(&PlayState::Idle), MONOKAI_CYAN);
    assert_eq!(
        status_color(&PlayState::Running("render".to_string())),
        MONOKAI_PURPLE
    );
    assert_eq!(
        status_color(&PlayState::Playing("play".to_string())),
        MONOKAI_YELLOW
    );
    assert_eq!(
        status_color(&PlayState::Done("done".to_string())),
        MONOKAI_GREEN
    );
    assert_eq!(status_color(&PlayState::Err("err".to_string())), Color::Red);
}

#[test]
fn normal_screen_splits_status_and_keybinds_without_line_numbers() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string()];

    let lines = render_lines(&mut app, 220, 9);
    let screen = lines.join("\n");
    let normalized_lines: Vec<String> = lines.iter().map(|line| line.replace(' ', "")).collect();
    let status_row = lines
        .iter()
        .position(|line| line.trim_start() == "NORMAL")
        .unwrap();
    let render_row = normalized_lines
        .iter()
        .position(|line| line.contains("render:実行0/2予約0"))
        .unwrap();
    let keybind_row = lines
        .iter()
        .position(|line| line.contains("q ?:help e:config b:loops"))
        .unwrap();

    assert!(screen.contains("[NORMAL] notepad mode"));
    assert!(screen.contains("▶   abc"));
    assert!(!screen.contains("MML Lines"));
    assert!(!screen.contains("▶   1 abc"));
    assert_eq!(render_row, status_row + 1);
    assert_eq!(keybind_row, render_row + 1);
    assert!(normalized_lines[render_row].contains("render:実行0/2予約0"));
    assert!(screen.contains("q ?:help e:config b:loops"));
    assert!(screen.contains("b:loops"));
    assert!(screen.contains("dd/Del:cut"));
    assert!(screen.contains("g:generate"));
    assert!(screen.contains("Shift+H:patch history"));
    assert!(!screen.contains("Shift+L:log"));
    assert!(!screen.contains("notepad r log"));
    assert!(!screen.contains("selected list"));
    assert!(screen.contains("w:DAW"));
    assert!(screen.contains("v:keyboard"));
}

#[test]
fn keyboard_screen_shows_connecting_status_and_navigation() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::Keyboard;

    let screen = render_lines(&mut app, 90, 14).join("\n");

    assert!(screen.contains("[KEYBOARD] keyboard mode"));
    assert!(screen.contains("transport: SHM"));
    assert!(screen.contains("buffer: x4"));
    assert!(screen.contains("server: idle"));
    assert!(screen.contains("last send: -"));
    assert!(screen.contains("connecting..."));
    assert!(screen.contains("notes unavailable until ready"));
    assert!(screen.contains("s:transport"));
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
fn loop_browser_error_screen_shows_scan_guidance() {
    let mut app = TuiApp::new_for_test(test_config());
    app.handle_normal(KeyCode::Char('b'));

    let screen = render_lines(&mut app, 180, 10).join("\n");

    assert!(screen.contains("[LOOP BROWSER] WAV loops"));
    assert!(screen.contains("cmrt scan-loops"));
    assert!(screen.contains("loop browser"));
}

#[test]
fn keyboard_screen_shows_count_input_guide_until_navigation() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::Keyboard;
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
        crate::tui::PatchLoadState::Ready(patches),
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
    loading.mode = Mode::Keyboard;
    let screen = render_lines(&mut loading, 140, 12).join("\n");
    assert!(screen.replace(' ', "").contains("パッチを読み込み中..."));

    let mut error = TuiApp::new_for_test(test_config());
    error.patch_load_state = std::sync::Arc::new(std::sync::Mutex::new(
        crate::tui::PatchLoadState::Err("boom".to_string()),
    ));
    error.mode = Mode::Keyboard;
    let screen = render_lines(&mut error, 140, 12).join("\n");
    assert!(screen.replace(' ', "").contains("読み込み失敗:boom"));

    let mut empty = TuiApp::new_for_test(test_config());
    empty.mode = Mode::Keyboard;
    let screen = render_lines(&mut empty, 140, 12).join("\n");
    assert!(screen.replace(' ', "").contains("パッチが見つかりません"));
}

#[test]
fn normal_screen_shows_active_parallel_render_count_in_purple() {
    let mut app = TuiApp::new_for_test(test_config());
    app.test_set_active_parallel_render_count(2);

    let buffer = render_buffer(&mut app, 120, 9);
    let (x, y) = find_text(&buffer, "render:");

    assert_eq!(buffer.cell((x, y)).unwrap().fg, MONOKAI_PURPLE);
}

#[test]
fn normal_screen_marks_cached_lines_with_music_note() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string(), "def".to_string()];
    app.audio_cache
        .lock()
        .unwrap()
        .insert("abc".to_string(), vec![0.1, 0.2]);

    let screen = render_lines(&mut app, 80, 8).join("\n");

    assert!(screen.contains("▶ ♪ abc"));
    assert!(screen.contains("  def"));
}

#[test]
fn normal_screen_marks_disk_only_cached_lines_with_music_note() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string(), "def".to_string()];
    // "abc" だけディスクキャッシュのハッシュ集合に入っている状態を模擬する。
    // audio_cache（オンメモリ）には積まない ＝ LRUから追い出された後の状態に相当する。
    app.known_disk_cache_hashes
        .lock()
        .unwrap()
        .insert(crate::history::daw_cache_mml_hash("abc"));

    let screen = render_lines(&mut app, 80, 8).join("\n");

    assert!(screen.contains("▶ ♪ abc"));
    assert!(screen.contains("  def"));
}

#[test]
fn normal_mode_startup_prime_caches_current_line_and_navigation_targets() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![
        "m0".to_string(),
        "m1".to_string(),
        "m2".to_string(),
        "m3".to_string(),
    ];
    app.cursor = 1;
    app.list_state.select(Some(1));
    app.normal_page_size = 2;

    app.prime_normal_mode_startup_cache();

    let cache = app.audio_cache.lock().unwrap();
    assert!(cache.contains_key("m1"));
    assert!(cache.contains_key("m2"));
    assert!(cache.contains_key("m0"));
    assert!(cache.contains_key("m3"));
}

#[test]
fn normal_screen_marks_rendering_lines_with_dots_before_music_note() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string()];
    app.test_set_render_job_status(
        "abc",
        Some(crate::tui::render_queue::TuiRenderJobStatus::Pending),
    );

    let screen = render_lines(&mut app, 80, 8).join("\n");

    assert!(screen.contains("▶ . abc"));
}

#[test]
fn insert_screen_shows_insert_title_without_duplicate_line_text() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string()];
    app.start_insert();

    let lines = render_lines(&mut app, 80, 8);
    let screen = lines.join("\n");

    assert!(screen.contains("[INSERT] notepad mode"));
    assert_eq!(screen.matches("abc").count(), 1);
    assert!(lines.iter().any(|line| line.contains("▶ abc")));
}

#[test]
fn patch_phrase_screen_uses_monokai_foreground_for_unfocused_list() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::PatchPhrase;
    app.patch_phrase_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_store.patches.insert(
        "Pads/Pad 1.fxp".to_string(),
        PatchPhraseState {
            history: vec!["l8cdef".to_string()],
            favorites: vec!["o5g".to_string()],
        },
    );

    let buffer = render_buffer(&mut app, 80, 10);
    let (x, y) = find_text(&buffer, "o5g");

    assert_eq!(buffer.cell((x, y)).unwrap().fg, MONOKAI_FG);
    assert_eq!(buffer.cell((x, y)).unwrap().bg, MONOKAI_BG);
}
