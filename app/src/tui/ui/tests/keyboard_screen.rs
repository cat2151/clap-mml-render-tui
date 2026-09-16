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
fn keyboard_screen_shows_role_preset_and_patch_panes_while_connecting() {
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

    let screen = render_lines(&mut app, 200, 14).join("\n");

    assert!(screen.contains("Role (1/7)"), "{screen}");
    assert!(screen.contains("Bass track"), "{screen}");
    assert!(screen.contains("Preset (1/"), "{screen}");
    assert!(screen.contains("Patches (3/3)"), "{screen}");
    assert!(screen.contains("Category"), "{screen}");
    assert!(screen.contains("Load"), "{screen}");
    assert!(
        screen.contains("patches_factory/Pad/Factory Pad.fxp"),
        "{screen}"
    );
    assert!(screen.contains("connecting..."));
    assert!(screen.contains("h/l:pane"));
    assert!(screen.contains("k/j/Up/Down:-/+1"));
    assert!(screen.contains("Ctrl+u/d/PgUp/PgDn:-/+10"));
    assert!(screen.contains("Home/End:first/last"));
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

/// 実 catalog cache（`cmrt build-patch-catalog-cache` の生成物）へのパス。無ければ下のテストは何もしない。
const REAL_CATALOG_ENV: &str = "CMRT_TEST_PATCH_CATALOG_JSON";

/// 実 catalog は preset label に `›` のような幅 1 の非 ASCII を含み、件数も合成データより
/// 3 桁多い。200 桁で 4 pane の枠が崩れず、件数が共有の Role 索引と一致することを見る。
#[test]
fn keyboard_panes_render_the_real_catalog_cache_at_200_columns() {
    let Some(path) = std::env::var_os(REAL_CATALOG_ENV) else {
        return;
    };
    let (snapshot, _) = crate::patch_catalog_cache::load_from(std::path::Path::new(&path))
        .unwrap()
        .into_parts();
    let total = snapshot.pairs().len();
    let bass = snapshot
        .patch_roles()
        .candidates(cmrt_patches::PatchRole::Bass)
        .len();
    let first = snapshot.pairs()[0].0.clone();
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_load_state = std::sync::Arc::new(std::sync::Mutex::new(
        crate::tui::PatchLoadState::Ready(std::sync::Arc::new(snapshot)),
    ));
    app.start_keyboard(Some(first));

    let assert_frames_intact = |app: &mut TuiApp<'static>| {
        let buffer = render_buffer(app, 200, 30);
        let symbol = |x: u16, y: u16| buffer.cell((x, y)).unwrap().symbol().to_string();
        assert_eq!(symbol(73, 0), "┐");
        for x in [74, 96, 126] {
            assert_eq!(symbol(x, 0), "┌", "x={x}");
        }
        assert_eq!(symbol(199, 0), "┐");
        let body_rows = (1..30)
            .take_while(|&y| symbol(199, y) == "│")
            .inspect(|&y| {
                for x in [0, 73, 74, 96, 126] {
                    assert_eq!(symbol(x, y), "│", "x={x} y={y}");
                }
            })
            .count();
        assert!(body_rows >= 5, "{body_rows}");
    };

    assert_frames_intact(&mut app);
    let screen = render_lines(&mut app, 200, 30).join(
        "
",
    );
    assert!(screen.contains("Role (1/7)"), "{screen}");
    assert!(screen.contains(&format!("/{total})")), "{screen}");
    assert!(screen.contains("Bass › bass|bs"), "{screen}");

    for key in ['h', 'h', 'j'] {
        app.handle_keyboard_key_event(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE));
    }
    assert_frames_intact(&mut app);
    let screen = render_lines(&mut app, 200, 30).join(
        "
",
    );
    assert!(screen.contains("Role (2/7)"), "{screen}");
    assert!(screen.contains(&format!("/{bass})")), "{screen}");
}
