use super::*;
use crate::loop_browser_metadata::LoopBrowserMetadata;
use crate::loop_library::{LoopIndex, LoopRootIndex, LoopWavIndex};
use crate::loop_wav_analysis::{
    LoopAnalysisSource, LoopTempoAnalysis, LoopWavAnalysis, LoopWavKind,
};
use crate::loop_waveform::{LoopWaveform, WAVEFORM_BINS_PER_MEASURE};
use crate::tui::loop_browser::LoopBrowser;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

fn browser_with_waveform(measures: usize) -> LoopBrowser {
    let levels = [-800, -700, -600, -500, -400, -300, -200, -100];
    let rms_db_tenths = (0..measures)
        .flat_map(|measure| {
            std::iter::repeat_n(levels[measure % levels.len()], WAVEFORM_BINS_PER_MEASURE)
        })
        .collect();
    LoopBrowser::from_index(
        LoopIndex {
            version: crate::loop_library::LOOP_INDEX_VERSION,
            roots: vec![LoopRootIndex {
                path: "/loops".to_string(),
                wav_files: vec![LoopWavIndex {
                    relative: "Long.wav".to_string(),
                    analysis: LoopWavAnalysis {
                        duration_seconds: 2.0 * measures as f64,
                        kind: LoopWavKind::Loop,
                        tempo: Some(LoopTempoAnalysis {
                            bpm: 120.0,
                            declared_bpm: Some(120.0),
                            beats: 4 * measures as u32,
                            meter_numerator: 4,
                            meter_denominator: 4,
                            source: LoopAnalysisSource::Acid,
                        }),
                        measures,
                    },
                    waveform: LoopWaveform {
                        rms_db_tenths,
                        spectral_flux: vec![0; measures * WAVEFORM_BINS_PER_MEASURE],
                        centroid_motion_millioctaves: 150,
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

fn place_selected_wav(app: &mut TuiApp<'_>) {
    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser
        .state
        .handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    app.loop_browser.state.handle_key(KeyCode::Tab);
    app.loop_browser.state.handle_key(KeyCode::Char('c'));
}

#[test]
fn draws_continuous_eight_measure_waveform_across_scrolled_cells() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::LoopBrowser;
    app.loop_browser.state = browser_with_waveform(8);
    place_selected_wav(&mut app);

    let first = render_lines(&mut app, 180, 20).join("\n");
    assert!(first.contains("[RMS / SPECTRAL MOTION]"));
    assert!(first.contains("1   2   3   4"));
    for glyph in ['▁', '▂', '▃', '▄'] {
        assert!(first.contains(&glyph.to_string().repeat(16)), "{first}");
    }

    app.loop_browser.state.handle_key(KeyCode::Char('7'));
    app.loop_browser.state.handle_key(KeyCode::Char('l'));
    let last = render_lines(&mut app, 180, 20).join("\n");
    for glyph in ['▅', '▆', '▇', '█'] {
        assert!(last.contains(&glyph.to_string().repeat(16)), "{last}");
    }
}

#[test]
fn offscreen_playback_keeps_edit_scroll_and_reports_measure_and_beat_in_title() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::LoopBrowser;
    app.loop_browser.state = browser_with_waveform(8);
    place_selected_wav(&mut app);
    app.loop_browser.state.set_playback_beat_for_test(7, 2, 4);

    let screen = render_lines(&mut app, 100, 20).join("\n");

    assert!(screen.contains("PLAY M8 B3"));
    assert_eq!(app.loop_browser.state.measure_scroll, 0);
}

#[test]
fn compact_height_hides_stretch_but_keeps_one_waveform_track() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::LoopBrowser;
    app.loop_browser.state = browser_with_waveform(2);
    place_selected_wav(&mut app);

    let compact = render_lines(&mut app, 180, 14).join("\n");
    assert!(compact.contains("[RMS / SPECTRAL MOTION]"));
    assert!(compact.contains(&"▁".repeat(16)));
    assert!(!compact.contains("[USED WAV / ANALYSIS"));

    let still_compact = render_lines(&mut app, 180, 17).join("\n");
    assert!(!still_compact.contains("[USED WAV / ANALYSIS"));

    let regular = render_lines(&mut app, 180, 18).join("\n");
    assert!(regular.contains("[USED WAV / ANALYSIS / CATEGORY / STRETCH SOURCE→TARGET / OUTPUT]"));
}

#[test]
fn waveform_keeps_green_theme_foreground_under_cursor_highlight() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::LoopBrowser;
    app.loop_browser.state = browser_with_waveform(2);
    place_selected_wav(&mut app);

    let buffer = render_buffer(&mut app, 180, 20);
    let cell = (0..buffer.area.height)
        .flat_map(|y| (0..buffer.area.width).map(move |x| (x, y)))
        .find_map(|position| buffer.cell(position).filter(|cell| cell.symbol() == "▁"))
        .expect("waveform glyph should be rendered");
    let ratatui::style::Color::Rgb(red, green, blue) = cell.fg else {
        panic!("waveform should use an RGB theme color");
    };
    assert!(green > red && green > blue);
    assert_eq!(cell.bg, cursor_highlight_bg(cell.fg));
}
