use ratatui::{backend::TestBackend, buffer::Buffer, style::Color, Terminal};

use crate::{config::Config, history::PatchPhraseState, tui::TuiApp};

use super::{
    draw, status_color, Mode, PlayState, MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY,
    MONOKAI_GREEN, MONOKAI_PURPLE, MONOKAI_YELLOW,
};

fn test_config() -> Config {
    Config {
        plugin_path: "/tmp/Surge XT.clap".to_string(),
        input_midi: "input.mid".to_string(),
        output_midi: "output.mid".to_string(),
        output_wav: "output.wav".to_string(),
        sample_rate: 44_100.0,
        buffer_size: 512,
        patch_path: None,
        patches_dir: Some("/tmp/patches".to_string()),
        daw_tracks: 9,
        daw_measures: 8,
    }
}

fn render_lines(app: &mut TuiApp<'static>, width: u16, height: u16) -> Vec<String> {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(app, f)).unwrap();
    let buffer = terminal.backend().buffer().clone();
    (0..height)
        .map(|y| {
            (0..width)
                .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

fn render_buffer(app: &mut TuiApp<'static>, width: u16, height: u16) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(app, f)).unwrap();
    terminal.backend().buffer().clone()
}

fn find_text(buffer: &Buffer, text: &str) -> (u16, u16) {
    for y in 0..buffer.area.height {
        let line: String = (0..buffer.area.width)
            .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
            .collect();
        if let Some(x) = line.find(text) {
            return (x as u16, y);
        }
    }
    panic!("text not found in buffer: {text}");
}

#[test]
fn patch_phrase_screen_renders_history_and_favorites_lists() {
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
    app.patch_phrase_history_state.select(Some(0));
    app.patch_phrase_favorites_state.select(Some(0));

    let lines = render_lines(&mut app, 80, 10).join("\n");

    assert!(lines.contains("History - Pads/Pad 1.fxp"));
    assert!(lines.contains("Favorites"));
    assert!(lines.contains("l8cdef"));
    assert!(lines.contains("o5g"));
}

#[test]
fn patch_phrase_screen_splits_status_and_keybinds() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::PatchPhrase;

    let lines = render_lines(&mut app, 120, 10);
    let normalized_lines: Vec<String> = lines.iter().map(|line| line.replace(' ', "")).collect();
    let normalized_status = "patch phrase".replace(' ', "");
    let status_row = normalized_lines
        .iter()
        .position(|line| line == &normalized_status)
        .unwrap();
    let keybind_row = normalized_lines
        .iter()
        .position(|line| line.contains("j/k・↑↓:再生移動PgUp/PgDn:1画面移動"))
        .unwrap();

    assert_eq!(keybind_row, status_row + 1);
}

#[test]
fn patch_select_screen_renders_as_overlay_on_normal_screen() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec![r#"{"Surge XT patch":"Pads/Pad 1.fxp"} abc"#.to_string()];
    app.patch_all = vec![
        ("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string()),
        (
            "Leads/Lead 1.fxp".to_string(),
            "leads/lead 1.fxp".to_string(),
        ),
    ];
    app.patch_filtered = vec!["Pads/Pad 1.fxp".to_string(), "Leads/Lead 1.fxp".to_string()];
    app.patch_list_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    let lines = render_lines(&mut app, 80, 16).join("\n");
    let normalized = lines.replace(' ', "");

    assert!(lines.contains("[PATCH SELECT] notepad mode"));
    assert!(lines.contains("▶ {\"Surge XT patch\":\"Pads/Pad 1.fxp\"} abc"));
    assert!(normalized.contains("音色選択-検索"));
    assert!(normalized.contains("パッチ(2/2)"));
    assert!(lines.contains("Pads/Pad 1.fxp"));
    assert!(lines.contains("Leads/Lead 1.fxp"));
}

#[test]
fn patch_select_screen_splits_status_and_keybinds() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string()];
    app.patch_all = vec![("Pads/Pad 1.fxp".to_string(), "pads/pad 1.fxp".to_string())];
    app.patch_filtered = vec!["Pads/Pad 1.fxp".to_string()];
    app.patch_list_state.select(Some(0));
    app.mode = Mode::PatchSelect;

    let lines = render_lines(&mut app, 120, 16);
    let normalized_lines: Vec<String> = lines.iter().map(|line| line.replace(' ', "")).collect();
    let status_row = normalized_lines
        .iter()
        .position(|line| {
            line.contains("音色選択") && !line.contains("検索") && !line.contains("決定")
        })
        .unwrap();
    let keybind_row = normalized_lines
        .iter()
        .position(|line| line.contains("Enter:決定ESC:キャンセル"))
        .unwrap();

    assert!(!normalized_lines[status_row].contains("Enter:決定"));
    assert_eq!(keybind_row, status_row + 1);
    assert!(normalized_lines[keybind_row].contains("Ctrl+F:お気に入り"));
    assert!(normalized_lines[keybind_row].contains("Ctrl+J/Ctrl+N・Ctrl+K/Ctrl+P"));
}

#[test]
fn notepad_history_overlay_renders_history_and_favorites_lists() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["before".to_string()];
    app.patch_phrase_store.notepad = PatchPhraseState {
        history: vec!["l8cdef".to_string()],
        favorites: vec!["o5g".to_string()],
    };
    app.start_notepad_history();

    let lines = render_lines(&mut app, 100, 16).join("\n");

    assert!(lines.contains("[HISTORY] notepad mode"));
    assert!(lines.contains("History"));
    assert!(lines.contains("Favorites"));
    assert!(lines.contains("l8cdef"));
    assert!(lines.contains("o5g"));
}

#[test]
fn notepad_history_overlay_shows_left_right_pane_keybinds() {
    let mut app = TuiApp::new_for_test(test_config());
    app.patch_phrase_store.notepad = PatchPhraseState {
        history: vec!["l8cdef".to_string()],
        favorites: vec!["o5g".to_string()],
    };
    app.start_notepad_history();

    let lines = render_lines(&mut app, 120, 16);
    let normalized_lines: Vec<String> = lines.iter().map(|line| line.replace(' ', "")).collect();

    assert!(normalized_lines
        .iter()
        .any(|line| line.contains("h/l・←/→:ペイン移動")));
}

#[test]
fn patch_phrase_screen_uses_c_as_fallback_for_empty_lists() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::PatchPhrase;
    app.patch_phrase_name = Some("Pads/Pad 1.fxp".to_string());
    app.patch_phrase_history_state.select(Some(0));
    app.patch_phrase_favorites_state.select(Some(0));

    let lines = render_lines(&mut app, 80, 10).join("\n");

    assert!(lines.contains("▶ c"));
}

#[test]
fn help_screen_mentions_ctrl_clipboard_shortcuts() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::Help;

    let lines = render_lines(&mut app, 120, 60);
    let screen = lines.join("\n");
    let normalized_screen = screen.replace([' ', '\n'], "");
    let normalized_lines: Vec<String> = lines.iter().map(|line| line.replace(' ', "")).collect();

    assert!(screen.contains("[HELP] notepad mode"));
    assert!(normalized_lines
        .iter()
        .any(|line| line.contains("Ctrl+C:コピー")));
    assert!(normalized_lines
        .iter()
        .any(|line| line.contains("K/?:ヘルプ(このページ)")));
    assert!(normalized_lines
        .iter()
        .any(|line| line.contains("Ctrl+X:カット")));
    assert!(normalized_lines
        .iter()
        .any(|line| line.contains("Ctrl+V:ペースト")));
    assert!(normalized_lines
        .iter()
        .any(|line| line.contains("Ctrl+F:現在音色とMMLをFavorites追加")));
    assert!(normalized_lines
        .iter()
        .any(|line| line.contains("Ctrl+J/Ctrl+N/↓:下へ移動")));
    assert!(normalized_lines
        .iter()
        .any(|line| line.contains("PageUp/PageDown:1画面移動")));
    assert!(normalized_lines
        .iter()
        .any(|line| line.contains("h:notepadhistory")));
    assert!(normalized_screen.contains("h/l・←/→:ペイン切替"));
    assert!(!normalized_lines
        .iter()
        .any(|line| line.contains("Ctrl+C:強制終了")));
}

#[test]
fn normal_screen_uses_monokai_background_and_cursor_color() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string()];

    let buffer = render_buffer(&mut app, 80, 8);

    assert_eq!(buffer.cell((0, 0)).unwrap().fg, MONOKAI_CYAN);
    assert_eq!(buffer.cell((0, 0)).unwrap().bg, MONOKAI_BG);
    assert_eq!(buffer.cell((4, 6)).unwrap().fg, MONOKAI_CYAN);
    assert_eq!(buffer.cell((4, 6)).unwrap().bg, MONOKAI_BG);
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

    let lines = render_lines(&mut app, 80, 8);
    let screen = lines.join("\n");

    assert!(screen.contains("[NORMAL] notepad mode"));
    assert!(screen.contains("▶ abc"));
    assert!(!screen.contains("MML Lines"));
    assert!(!screen.contains("▶   1 abc"));
    assert_eq!(lines[6].trim_start(), "NORMAL");
    assert!(lines[7].contains("q ?:help i:insert"));
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
