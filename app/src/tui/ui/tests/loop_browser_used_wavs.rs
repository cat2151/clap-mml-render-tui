use super::loop_browser::{
    browser_at_bpm, browser_at_bpm_with_metadata, browser_with_repeat_and_rejected_cells,
};
use super::*;
use crate::loop_browser::metadata::{LoopBrowserMetadata, LoopDirId};
use crate::tui::loop_browser::LoopGridChange;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::Path;
use std::sync::Arc;

#[test]
fn stretch_result_pane_shows_prepared_output_duration() {
    use crate::loop_browser::time_stretch::PreparedAudioInfo;
    use crate::tui::loop_browser::playback::diagnostics::StretchStatus;

    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::LoopBrowser;
    app.loop_browser.state = browser_at_bpm(100.0);
    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser.state.handle_key(KeyCode::Char('l'));
    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser
        .state
        .handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    app.loop_browser.state.handle_key(KeyCode::Tab);
    app.loop_browser.state.handle_key(KeyCode::Char('c'));
    let grid = app.loop_browser.state.playback_grid();
    let clip = grid[0][0].as_ref().unwrap().clone();
    let target = app.loop_browser.state.target_bpm();
    let diagnostics = Arc::clone(&app.loop_browser.state.stretch_diagnostics);
    let mut diagnostics = diagnostics.lock().unwrap();
    diagnostics.begin(7, LoopGridChange::Pad('c'), &grid, target);
    diagnostics.set_status(
        7,
        &clip,
        target.bpm,
        StretchStatus::Ready {
            info: PreparedAudioInfo {
                input_frames: 100,
                rubberband_output_frames: 83,
                output_frames: 83,
                channels: 2,
                sample_rate: 50,
                time_ratio: 100.0 / 120.0,
                profile: rubberband_ffi::StretchProfile::General,
            },
            cache_hit: false,
        },
    );
    drop(diagnostics);

    let screen = render_lines(&mut app, 180, 20).join("\n");

    assert!(screen.contains("none"));
    assert!(screen.contains("✓ 100→120"));
}

#[test]
fn used_wav_pane_lists_a_spanning_wav_once_with_its_category() {
    let mut metadata = LoopBrowserMetadata::default();
    metadata.toggle_category(
        &LoopDirId::new(Path::new("/loops"), Path::new("Pack")),
        "drum",
    );
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::LoopBrowser;
    app.loop_browser.state = browser_at_bpm_with_metadata(120.0, metadata);
    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser.state.handle_key(KeyCode::Char('l'));
    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser
        .state
        .handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    app.loop_browser.state.handle_key(KeyCode::Tab);
    app.loop_browser.state.handle_key(KeyCode::Char('c'));

    let screen = render_lines(&mut app, 180, 20).join("\n");

    assert_eq!(
        screen
            .lines()
            .filter(|line| line.contains("[BPM") && line.contains("drum"))
            .count(),
        1
    );
    assert!(screen.contains("? 120→120"));
}

#[test]
fn used_wav_pane_groups_by_wav_and_adds_measures_for_multiple_wavs() {
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::LoopBrowser;
    app.loop_browser.state = browser_with_repeat_and_rejected_cells();

    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser
        .state
        .handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser
        .state
        .handle_key_event(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT));
    app.loop_browser.state.handle_key(KeyCode::Tab);
    app.loop_browser.state.handle_key(KeyCode::Char('c'));
    app.loop_browser.state.handle_key(KeyCode::Char('l'));
    app.loop_browser.state.handle_key(KeyCode::Char('d'));
    app.loop_browser.state.handle_key(KeyCode::Char('9'));
    app.loop_browser.state.handle_key(KeyCode::Char('l'));

    let screen = render_lines(&mut app, 180, 20).join("\n");
    let used_rows = screen
        .lines()
        .filter(|line| line.contains("T1") && line.contains("[BPM"))
        .collect::<Vec<_>>();

    assert!(screen.contains("track  meas"), "{screen}");
    assert_eq!(used_rows.len(), 2, "{screen}");
    assert!(used_rows.iter().any(|line| line.contains("A-good.wav")));
    assert!(used_rows
        .iter()
        .any(|line| line.contains("C-bad-short.wav")));
    assert!(app.loop_browser.state.measure_scroll > 0);
}
