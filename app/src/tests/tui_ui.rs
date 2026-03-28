use ratatui::{backend::TestBackend, Terminal};

use crate::{config::Config, history::PatchPhraseState, tui::TuiApp};

use super::{draw, Mode};

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

    let normalized_lines: Vec<String> = render_lines(&mut app, 80, 30)
        .into_iter()
        .map(|line| line.replace(' ', ""))
        .collect();

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
    assert!(!normalized_lines
        .iter()
        .any(|line| line.contains("Ctrl+C:強制終了")));
}

#[test]
fn normal_screen_splits_status_and_keybinds_without_line_numbers() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string()];

    let lines = render_lines(&mut app, 80, 8);
    let screen = lines.join("\n");

    assert!(screen.contains("▶ abc"));
    assert!(!screen.contains("MML Lines"));
    assert!(!screen.contains("▶   1 abc"));
    assert_eq!(lines[6].trim_start(), "NORMAL");
    assert!(lines[7].contains("q:quit  ?:help  i:insert"));
}

#[test]
fn insert_screen_shows_insert_title_without_duplicate_line_text() {
    let mut app = TuiApp::new_for_test(test_config());
    app.lines = vec!["abc".to_string()];
    app.start_insert();

    let screen = render_lines(&mut app, 80, 8).join("\n");

    assert!(screen.contains("[INSERT]"));
    assert_eq!(screen.matches("abc").count(), 1);
}
