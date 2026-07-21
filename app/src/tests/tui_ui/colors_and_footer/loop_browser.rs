use super::*;
use crate::loop_browser_metadata::{LoopBrowserMetadata, LoopDirId};
use crate::loop_library::{LoopIndex, LoopRootIndex, LoopWavIndex};
use crate::loop_wav_analysis::{
    LoopAnalysisSource, LoopTempoAnalysis, LoopWavAnalysis, LoopWavKind,
};
use crate::loop_waveform::LoopWaveform;
use crate::tui::loop_browser::{LoopBrowser, LoopGridChange};
use crate::tui::NormalAction;
use std::path::Path;
use std::sync::Arc;

fn browser() -> LoopBrowser {
    browser_at_bpm(120.0)
}

fn browser_at_bpm(bpm: f64) -> LoopBrowser {
    browser_at_bpm_with_metadata(bpm, LoopBrowserMetadata::default())
}

fn browser_at_bpm_with_metadata(bpm: f64, metadata: LoopBrowserMetadata) -> LoopBrowser {
    LoopBrowser::from_index(
        LoopIndex {
            version: crate::loop_library::LOOP_INDEX_VERSION,
            roots: vec![LoopRootIndex {
                path: "/loops".to_string(),
                wav_files: vec![LoopWavIndex {
                    relative: "Pack/Kick.wav".to_string(),
                    analysis: LoopWavAnalysis {
                        duration_seconds: 4.0,
                        kind: LoopWavKind::Loop,
                        tempo: Some(LoopTempoAnalysis {
                            bpm,
                            declared_bpm: Some(bpm),
                            beats: 8,
                            meter_numerator: 4,
                            meter_denominator: 4,
                            source: LoopAnalysisSource::Acid,
                        }),
                        measures: 2,
                    },
                    waveform: LoopWaveform::silent(2),
                }],
            }],
        },
        &crate::config::default_loop_categories(),
        metadata,
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
            kind: LoopWavKind::Loop,
            tempo: Some(LoopTempoAnalysis {
                bpm,
                declared_bpm: Some(bpm),
                beats: 4 * measures as u32,
                meter_numerator: 4,
                meter_denominator: 4,
                source: LoopAnalysisSource::Acid,
            }),
            measures,
        },
        waveform: LoopWaveform::silent(measures),
    };
    LoopBrowser::from_index(
        LoopIndex {
            version: crate::loop_library::LOOP_INDEX_VERSION,
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

fn browser_with_many_direct_wavs(count: usize) -> LoopBrowser {
    LoopBrowser::from_index(
        LoopIndex {
            version: crate::loop_library::LOOP_INDEX_VERSION,
            roots: vec![LoopRootIndex {
                path: "/loops".to_string(),
                wav_files: (0..count)
                    .map(|index| LoopWavIndex {
                        relative: format!("{index:04}.wav"),
                        analysis: LoopWavAnalysis {
                            duration_seconds: 2.0,
                            kind: LoopWavKind::Loop,
                            tempo: Some(LoopTempoAnalysis {
                                bpm: 120.0,
                                declared_bpm: Some(120.0),
                                beats: 4,
                                meter_numerator: 4,
                                meter_denominator: 4,
                                source: LoopAnalysisSource::Acid,
                            }),
                            measures: 1,
                        },
                        waveform: LoopWaveform::silent(1),
                    })
                    .collect(),
            }],
        },
        &crate::config::default_loop_categories(),
        LoopBrowserMetadata::default(),
        None,
        true,
        None,
    )
}

#[path = "loop_browser/used_wavs.rs"]
mod used_wavs;

#[test]
fn error_screen_shows_scan_guidance() {
    let mut app = TuiApp::new_for_test(test_config());
    let action = app.handle_normal(KeyCode::Char('b'));
    assert!(matches!(action, NormalAction::LaunchLoopBrowser));
    app.begin_loop_browser_startup();

    let loading = render_lines(&mut app, 180, 10).join("\n");
    assert!(
        loading.replace(' ', "").contains("LoopBrowser起動中…"),
        "{loading}"
    );

    app.complete_loop_browser_startup();

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
    app.loop_browser.state = browser();
    app.loop_browser.state.cursor = 1;

    app.loop_browser.state.handle_key(KeyCode::Char('v'));
    let favorite_screen = render_lines(&mut app, 180, 12).join("\n");
    assert!(favorite_screen.contains("★"));

    app.loop_browser.state.handle_key(KeyCode::Char('V'));
    let favorites_only_screen = render_lines(&mut app, 180, 12).join("\n");
    assert!(favorites_only_screen.contains("Favorite dirs"));
    app.loop_browser.state.handle_key(KeyCode::Char('V'));

    app.loop_browser.state.handle_key(KeyCode::Char('t'));
    let category_screen = render_lines(&mut app, 180, 12).join("\n");
    assert!(category_screen.contains("g: guitar"));
    assert!(category_screen.contains("e: sequence"));
    app.loop_browser.state.handle_key(KeyCode::Esc);

    app.loop_browser.state.handle_key(KeyCode::Char('v'));
    let notice_buffer = render_buffer(&mut app, 180, 12);
    find_text_ignoring_spaces(&notice_buffer, "お気に入りdirを解除しました");
}

#[test]
fn draws_wav_pads_track_grid_and_pane_specific_footer() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::LoopBrowser;
    app.loop_browser.state = browser();
    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser.state.handle_key(KeyCode::Char('l'));
    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser
        .state
        .handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    app.loop_browser.state.handle_key(KeyCode::Tab);
    app.loop_browser.state.handle_key(KeyCode::Char('c'));

    let screen = render_lines(&mut app, 180, 20).join("\n");

    assert!(screen.contains("[WAV PADS]"));
    assert!(screen.contains("[TRACK LIST BPM120 AUTO-STRETCH]"));
    assert!(screen.contains("[USED WAV / ANALYSIS / CATEGORY / STRETCH SOURCE→TARGET / OUTPUT]"));
    assert!(screen.contains("C:Kick.wav"));
    assert!(screen.contains("[BPM120 beat8 2meas]"));
    assert!(screen.contains("? 120→120"));
    assert!(screen.contains("none"));
    assert!(screen.contains("↳ 2/2"));
    assert!(screen.contains("Tab:loop tree"));
    assert!(screen.contains("m:mix level"));
    assert!(screen.replace(' ', "").contains("p:演奏停止/再開"));
    assert!(screen.replace(' ', "").contains("r:ランダムWAV"));
    assert!(screen.contains("1-9:hjkl prefix"));
}

#[test]
fn tree_draws_only_viewport_rows_and_keeps_a_quarter_margin() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::LoopBrowser;
    app.loop_browser.state = browser_with_many_direct_wavs(7_000);
    app.loop_browser.state.cursor = 3_500;
    app.loop_browser
        .state
        .handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    app.loop_browser.state.handle_key(KeyCode::Tab);
    app.loop_browser.state.handle_key(KeyCode::Char('c'));

    let _ = render_lines(&mut app, 120, 20);

    let metrics = app.loop_browser.state.last_render_metrics.unwrap();
    assert_eq!(metrics.total_tree_nodes, 7_001);
    assert_eq!(metrics.rendered_tree_nodes, 12);
    assert_eq!(app.loop_browser.state.tree_scroll, 3_492);
    assert!(
        metrics.tracks < std::time::Duration::from_millis(100),
        "track pane render exceeded budget: {:?}",
        metrics.tracks
    );
}

#[test]
fn breadcrumb_shows_selected_wavs_parent_directory() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::LoopBrowser;
    app.loop_browser.state = browser();
    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser.state.handle_key(KeyCode::Char('l'));
    app.loop_browser.state.handle_key(KeyCode::Char('j'));

    let screen = render_lines(&mut app, 120, 14).join("\n");

    assert!(screen.contains("loops › Pack"));
    assert!(!screen.contains("loops › Pack › Kick.wav"));
}

#[test]
fn breadcrumb_shows_the_selected_wavs_parent_direct_category() {
    let mut metadata = LoopBrowserMetadata::default();
    metadata.toggle_category(
        &LoopDirId::new(Path::new("/loops"), Path::new("Pack")),
        "drum",
    );
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::LoopBrowser;
    app.loop_browser.state = browser_at_bpm_with_metadata(120.0, metadata);
    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser.state.handle_key(KeyCode::Char('l'));
    app.loop_browser.state.handle_key(KeyCode::Char('j'));

    let screen = render_lines(&mut app, 120, 14).join("\n");

    assert!(screen.contains("loops › Pack [drum]"), "{screen}");
}

#[test]
fn track_pane_shows_solo_and_mute_labels_and_shortcut() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::LoopBrowser;
    app.loop_browser.state = browser();
    app.loop_browser.state.handle_key(KeyCode::Tab);
    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser.state.handle_key(KeyCode::Char('s'));

    let screen = render_lines(&mut app, 180, 20).join("\n");

    assert!(screen.contains("mute T1"), "{screen}");
    assert!(screen.contains("solo T2"), "{screen}");
    assert!(screen.contains("s:solo toggle"), "{screen}");

    let buffer = render_buffer(&mut app, 180, 20);
    let (mute_x, mute_y) = find_text(&buffer, "mute T1");
    let (solo_x, solo_y) = find_text(&buffer, "solo T2");
    assert_eq!(buffer.cell((mute_x, mute_y)).unwrap().fg, MONOKAI_GRAY);
    assert_eq!(buffer.cell((solo_x, solo_y)).unwrap().fg, MONOKAI_GREEN);
    let muted_cell = (90..180)
        .find_map(|x| {
            let cell = buffer.cell((x, mute_y)).unwrap();
            (cell.symbol() == "·").then_some(cell)
        })
        .expect("muted track cell should be visible");
    assert_eq!(muted_cell.fg, MONOKAI_GRAY);
}

#[test]
fn paused_loop_browser_has_an_explicit_status() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::LoopBrowser;
    app.loop_browser.state = browser();
    app.loop_browser.state.handle_key(KeyCode::Char('p'));

    let screen = render_lines(&mut app, 180, 14).join("\n");

    assert!(screen.replace(' ', "").contains("⏸停止中"));
}

#[test]
fn track_title_shows_the_automatically_adjusted_bpm() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::LoopBrowser;
    app.loop_browser.state = browser_at_bpm(160.0);
    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser.state.handle_key(KeyCode::Char('l'));
    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser
        .state
        .handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    app.loop_browser.state.handle_key(KeyCode::Tab);
    app.loop_browser.state.handle_key(KeyCode::Char('c'));

    let screen = render_lines(&mut app, 180, 14).join("\n");

    assert!(screen.contains("[TRACK LIST BPM128 AUTO-STRETCH]"));
}

#[test]
fn trailing_repeats_are_gray_and_bpm_rejected_cells_are_red() {
    let mut app = TuiApp::new_for_test(test_config());
    app.mode = Mode::LoopBrowser;
    app.loop_browser.state = browser_with_repeat_and_rejected_cells();

    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser
        .state
        .handle_key_event(KeyEvent::new(KeyCode::Char('C'), KeyModifiers::SHIFT));
    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser
        .state
        .handle_key_event(KeyEvent::new(KeyCode::Char('D'), KeyModifiers::SHIFT));
    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser
        .state
        .handle_key_event(KeyEvent::new(KeyCode::Char('E'), KeyModifiers::SHIFT));
    app.loop_browser.state.handle_key(KeyCode::Tab);
    app.loop_browser.state.handle_key(KeyCode::Char('c'));
    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser.state.handle_key(KeyCode::Char('d'));
    app.loop_browser.state.handle_key(KeyCode::Char('j'));
    app.loop_browser.state.handle_key(KeyCode::Char('e'));
    app.loop_browser.state.handle_key(KeyCode::Char('l'));

    let buffer = render_buffer(&mut app, 180, 24);
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
    app.loop_browser.state = browser();
    app.loop_browser.state.handle_key(KeyCode::Tab);
    app.loop_browser.state.handle_key(KeyCode::Char('m'));
    app.loop_browser.state.handle_key(KeyCode::Char('k'));

    let screen = render_lines(&mut app, 100, 30).join("\n");

    assert!(screen.contains("mixer"));
    assert!(screen.contains("track1"));
    assert!(screen.contains("+3dB"));
    assert!(screen.contains("j/k: -/+3dB"));
}
