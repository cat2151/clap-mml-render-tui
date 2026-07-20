use super::*;
use crate::loop_browser_metadata::LoopBrowserMetadata;
use crate::loop_library::{LoopIndex, LoopRootIndex, LoopWavIndex};
use crate::loop_wav_analysis::{LoopAnalysisSource, LoopWavAnalysis};
use crate::tui::loop_browser::LoopBrowser;

fn browser() -> LoopBrowser {
    browser_at_bpm(120.0)
}

fn browser_at_bpm(bpm: f64) -> LoopBrowser {
    LoopBrowser::from_index(
        LoopIndex {
            version: 2,
            roots: vec![LoopRootIndex {
                path: "/loops".to_string(),
                wav_files: vec![LoopWavIndex {
                    relative: "Pack/Kick.wav".to_string(),
                    analysis: LoopWavAnalysis {
                        duration_seconds: 4.0,
                        bpm,
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

fn browser_with_repeat_and_rejected_cells() -> LoopBrowser {
    let wav = |relative: &str, bpm: f64, measures: usize| LoopWavIndex {
        relative: relative.to_string(),
        analysis: LoopWavAnalysis {
            duration_seconds: 2.0 * measures as f64,
            bpm,
            beats: 4 * measures as u32,
            meter_numerator: 4,
            meter_denominator: 4,
            measures,
            source: LoopAnalysisSource::Acid,
        },
    };
    LoopBrowser::from_index(
        LoopIndex {
            version: 2,
            roots: vec![LoopRootIndex {
                path: "/loops".to_string(),
                wav_files: vec![
                    wav("A-good.wav", 100.0, 1),
                    wav("B-bad-long.wav", 200.0, 2),
                    wav("C-bad-short.wav", 200.0, 1),
                ],
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
    assert!(screen.replace(' ', "").contains("r:ランダムWAV"));
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
    assert!(screen.contains("m:mix level"));
    assert!(screen.replace(' ', "").contains("r:ランダムWAV"));
    assert!(screen.contains("1-9:hjkl prefix"));
}

#[test]
fn track_title_shows_the_automatically_adjusted_bpm() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::LoopBrowser;
    app.loop_browser = browser_at_bpm(160.0);
    app.loop_browser.handle_key(KeyCode::Char('j'));
    app.loop_browser.handle_key(KeyCode::Char('l'));
    app.loop_browser.handle_key(KeyCode::Char('j'));
    app.loop_browser
        .handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    app.loop_browser.handle_key(KeyCode::Tab);
    app.loop_browser.handle_key(KeyCode::Char('c'));

    let screen = render_lines(&mut app, 180, 14).join("\n");

    assert!(screen.contains("[TRACK LIST BPM128 AUTO-STRETCH]"));
}

#[test]
fn trailing_repeats_are_gray_and_bpm_rejected_cells_are_red() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::LoopBrowser;
    app.loop_browser = browser_with_repeat_and_rejected_cells();

    app.loop_browser.handle_key(KeyCode::Char('j'));
    app.loop_browser
        .handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    app.loop_browser.handle_key(KeyCode::Char('j'));
    app.loop_browser
        .handle_key_event(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT));
    app.loop_browser.handle_key(KeyCode::Char('j'));
    app.loop_browser
        .handle_key_event(KeyEvent::new(KeyCode::Char('E'), KeyModifiers::SHIFT));
    app.loop_browser.handle_key(KeyCode::Tab);
    app.loop_browser.handle_key(KeyCode::Char('c'));
    app.loop_browser.handle_key(KeyCode::Char('j'));
    app.loop_browser.handle_key(KeyCode::Char('d'));
    app.loop_browser.handle_key(KeyCode::Char('j'));
    app.loop_browser.handle_key(KeyCode::Char('e'));
    app.loop_browser.handle_key(KeyCode::Char('l'));

    let buffer = render_buffer(&mut app, 180, 16);
    let symbols = |symbol: &str| {
        (0..buffer.area.height)
            .flat_map(|y| (0..buffer.area.width).map(move |x| (x, y)))
            .filter(|position| buffer.cell(*position).unwrap().symbol() == symbol)
            .collect::<Vec<_>>()
    };
    let repeats = symbols("↻");
    let continuations = symbols("↳");

    assert_eq!(repeats.len(), 2);
    assert_eq!(continuations.len(), 1);
    assert_eq!(buffer.cell(repeats[0]).unwrap().fg, MONOKAI_GRAY);
    assert_eq!(buffer.cell(repeats[1]).unwrap().fg, Color::Red);
    assert_eq!(
        buffer.cell(repeats[1]).unwrap().bg,
        cursor_highlight_bg(Color::Red)
    );
    let (continuation_x, continuation_y) = continuations[0];
    assert_eq!(
        buffer.cell((continuation_x, continuation_y)).unwrap().fg,
        Color::Red
    );
    assert_eq!(
        buffer
            .cell((continuation_x - 14, continuation_y))
            .unwrap()
            .fg,
        Color::Red
    );
}

#[test]
fn draws_shared_mixer_overlay_for_loop_tracks() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::LoopBrowser;
    app.loop_browser = browser();
    app.loop_browser.handle_key(KeyCode::Tab);
    app.loop_browser.handle_key(KeyCode::Char('m'));
    app.loop_browser.handle_key(KeyCode::Char('k'));

    let screen = render_lines(&mut app, 100, 30).join("\n");

    assert!(screen.contains("mixer"));
    assert!(screen.contains("track1"));
    assert!(screen.contains("+3dB"));
    assert!(screen.contains("j/k: -/+3dB"));
}
