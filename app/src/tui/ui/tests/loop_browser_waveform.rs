use super::*;
use crate::loop_browser::library::{LoopIndex, LoopRootIndex, LoopWavIndex};
use crate::loop_browser::metadata::LoopBrowserMetadata;
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
            version: crate::loop_browser::library::LOOP_INDEX_VERSION,
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
        crate::loop_browser::persisted::PersistedDoc::in_memory(LoopBrowserMetadata::default()),
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
    app.active_screen = crate::screen_switch::PrimaryScreen::LoopBrowser;
    app.loop_browser.state = browser_with_waveform(8);
    place_selected_wav(&mut app);

    // 8 小節が入りきらない幅で描く。セル幅は下限 8 まで詰まり、残りは横スクロールになる。
    let first = render_lines(&mut app, 100, 20).join("\n");
    assert!(first.contains("[RMS / SPECTRAL MOTION]"));
    assert!(first.contains("1 2 3 4"));
    for glyph in ['▁', '▂', '▃', '▄'] {
        assert!(first.contains(&glyph.to_string().repeat(8)), "{first}");
    }

    app.loop_browser.state.handle_key(KeyCode::Char('7'));
    app.loop_browser.state.handle_key(KeyCode::Char('l'));
    let last = render_lines(&mut app, 100, 20).join("\n");
    assert!(app.loop_browser.state.measure_scroll > 0);
    for glyph in ['▅', '▆', '▇', '█'] {
        assert!(last.contains(&glyph.to_string().repeat(8)), "{last}");
    }
}

/// 幅が足りている端末では 16 小節が横スクロールなしで全部並ぶ。
/// 440 桁なら右ペイン 264 / 内側 262 / track ラベルを引いて 253 桁、
/// これを 16 小節で割ってセル幅 15 になる。
#[test]
fn a_sixteen_measure_loop_fits_on_one_screen_without_horizontal_scrolling() {
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::LoopBrowser;
    app.loop_browser.state = browser_with_waveform(16);
    place_selected_wav(&mut app);

    let screen = render_lines(&mut app, 440, 20).join("\n");

    assert_eq!(app.loop_browser.state.measure_scroll, 0);
    assert!(screen.contains("measure 16"), "{screen}");
    assert!(screen.contains("↳ 16/16"), "{screen}");
    let waveform_row = screen
        .lines()
        .find(|line| line.contains("T1") && line.contains('▁'))
        .expect("waveform row should be drawn");
    assert_eq!(
        waveform_row
            .chars()
            .filter(|glyph| "▁▂▃▄▅▆▇█".contains(*glyph))
            .count(),
        16 * 15,
        "{waveform_row}"
    );
}

#[test]
fn offscreen_playback_keeps_edit_scroll_and_reports_measure_and_beat_in_title() {
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::LoopBrowser;
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
    app.active_screen = crate::screen_switch::PrimaryScreen::LoopBrowser;
    app.loop_browser.state = browser_with_waveform(2);
    place_selected_wav(&mut app);

    let compact = render_lines(&mut app, 180, 14).join("\n");
    assert!(compact.contains("[RMS / SPECTRAL MOTION]"));
    // 2 小節しかないループは上限まで広がり、1 文字 = 32 分音符になる。
    assert!(compact.contains(&"▁".repeat(32)), "{compact}");
    assert!(!compact.contains(&"▁".repeat(33)), "{compact}");
    assert!(!compact.contains("[USED WAV / ANALYSIS"));

    let still_compact = render_lines(&mut app, 180, 17).join("\n");
    assert!(!still_compact.contains("[USED WAV / ANALYSIS"));

    let regular = render_lines(&mut app, 180, 18).join("\n");
    assert!(regular.contains("[USED WAV / ANALYSIS / CATEGORY / STRETCH SOURCE→TARGET / OUTPUT]"));
}

#[test]
fn waveform_keeps_green_theme_foreground_under_cursor_highlight() {
    let mut app = TuiApp::new_for_test(test_config());
    app.active_screen = crate::screen_switch::PrimaryScreen::LoopBrowser;
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
