use super::*;
use crate::loop_browser_metadata::LoopBrowserMetadata;
use crate::loop_library::{LoopIndex, LoopRootIndex, LoopWavIndex};
use crate::loop_wav_analysis::{LoopAnalysisSource, LoopWavAnalysis};
use crate::tui::loop_browser::LoopBrowser;

fn browser() -> LoopBrowser {
    LoopBrowser::from_index(
        LoopIndex {
            version: 2,
            roots: vec![LoopRootIndex {
                path: "/loops".to_string(),
                wav_files: vec![LoopWavIndex {
                    relative: "Pack/Kick.wav".to_string(),
                    analysis: LoopWavAnalysis {
                        duration_seconds: 4.0,
                        bpm: 120.0,
                        beats: 8,
                        meter_numerator: 4,
                        meter_denominator: 4,
                        measures: 2,
                        source: LoopAnalysisSource::Acid,
                    },
                }],
            }],
        },
        &crate::config::default_loop_categories(),
        LoopBrowserMetadata::default(),
        None,
        true,
        None,
    )
}

#[test]
fn error_screen_shows_scan_guidance() {
    let mut app = TuiApp::new_for_test(test_config());
    app.handle_normal(KeyCode::Char('b'));

    let screen = render_lines(&mut app, 180, 10).join("\n");

    assert!(screen.contains("[LOOP TREE] WAV loops"));
    assert!(screen.contains("cmrt scan-loops"));
    assert!(screen.contains("loop browser"));
}

#[test]
fn draws_favorites_category_overlay_and_removal_notice() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::LoopBrowser;
    app.loop_browser = browser();
    app.loop_browser.cursor = 1;

    app.loop_browser.handle_key(KeyCode::Char('v'));
    let favorite_screen = render_lines(&mut app, 180, 12).join("\n");
    assert!(favorite_screen.contains("★"));

    app.loop_browser.handle_key(KeyCode::Char('V'));
    let favorites_only_screen = render_lines(&mut app, 180, 12).join("\n");
    assert!(favorites_only_screen.contains("Favorite dirs"));
    app.loop_browser.handle_key(KeyCode::Char('V'));

    app.loop_browser.handle_key(KeyCode::Char('t'));
    let category_screen = render_lines(&mut app, 180, 12).join("\n");
    assert!(category_screen.contains("g: guitar"));
    assert!(category_screen.contains("e: sequence"));
    app.loop_browser.handle_key(KeyCode::Esc);

    app.loop_browser.handle_key(KeyCode::Char('v'));
    let notice_buffer = render_buffer(&mut app, 180, 12);
    find_text_ignoring_spaces(&notice_buffer, "お気に入りdirを解除しました");
}

#[test]
fn draws_wav_pads_track_grid_and_pane_specific_footer() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::LoopBrowser;
    app.loop_browser = browser();
    app.loop_browser.handle_key(KeyCode::Char('j'));
    app.loop_browser.handle_key(KeyCode::Char('l'));
    app.loop_browser.handle_key(KeyCode::Char('j'));
    app.loop_browser
        .handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    app.loop_browser.handle_key(KeyCode::Tab);
    app.loop_browser.handle_key(KeyCode::Char('c'));

    let screen = render_lines(&mut app, 180, 14).join("\n");

    assert!(screen.contains("[WAV PADS]"));
    assert!(screen.contains("[TRACK LIST BPM120 AUTO-STRETCH]"));
    assert!(screen.contains("C:Kick.wav"));
    assert!(screen.contains("[BPM120 beat8 2meas]"));
    assert!(screen.contains("↳ 2/2"));
    assert!(screen.contains("Tab:loop tree"));
    assert!(screen.contains("1-9:hjkl prefix"));
}
