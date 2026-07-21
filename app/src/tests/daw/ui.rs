pub(super) use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
};

pub(super) use ratatui::{
    backend::TestBackend, buffer::Buffer, layout::Position, style::Color, Terminal,
};
pub(super) use tui_textarea::TextArea;

pub(super) use crate::config::Config;
pub(super) use crate::test_utils::{find_text_ignoring_spaces, help_overlay_bounds};
pub(super) use crate::ui_theme::cursor_highlight_bg;

pub(super) use super::{
    super::{
        AbRepeatState, CacheState, CellCache, DawApp, DawMode, DawPlayState, PlayPosition, MEASURES,
    },
    cache_indicator, cache_indicator_color, cache_text_color, draw, loop_measure_summary_label,
    loop_status_label, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY, MONOKAI_PINK,
};

fn build_test_app() -> DawApp {
    let tracks = 3;
    let measures = 2;
    let (cache_tx, _cache_rx) = std::sync::mpsc::channel();
    DawApp {
        editor: crate::daw::editor::DawEditorState::new(
            vec![vec![String::new(); measures + 1]; tracks],
            0,
            0,
            tracks,
            measures,
        ),
        mode: DawMode::Normal,
        help_origin: DawMode::Normal,
        sound_check_guide: crate::sound_check_guide::SoundCheckGuide::new(None),
        textarea: TextArea::default(),
        cfg: Arc::new(Config {
            plugin_path: String::new(),
            input_midi: String::new(),
            output_midi: String::new(),
            output_wav: String::new(),
            sample_rate: 44_100.0,
            buffer_size: 512,
            patches_dirs: None,
            loop_dirs: Vec::new(),
            loop_categories: crate::config::default_loop_categories(),
            offline_render_workers: crate::config::DEFAULT_OFFLINE_RENDER_WORKERS,
            offline_render_server_workers: crate::config::DEFAULT_OFFLINE_RENDER_SERVER_WORKERS,
            offline_render_backend: crate::config::OfflineRenderBackend::InProcess,
            offline_render_server_port: crate::config::DEFAULT_OFFLINE_RENDER_SERVER_PORT,
            offline_render_server_command: String::new(),
            realtime_audio_backend: crate::config::RealtimeAudioBackend::InProcess,
            realtime_play_server_port: crate::config::DEFAULT_REALTIME_PLAY_SERVER_PORT,
            realtime_play_server_command: String::new(),
            autoplay_on_startup: true,
            voicing_shared_source: String::new(),
            voicing_override_source: String::new(),
        }),
        entry_ptr: 0,
        cache: Arc::new(Mutex::new(vec![
            vec![CellCache::empty(); measures + 1];
            tracks
        ])),
        cache_tx,
        cache_render_workers: crate::config::DEFAULT_OFFLINE_RENDER_WORKERS,
        render_queue: crate::daw::render_queue::RenderQueue::disabled_for_tests(),
        playback: crate::daw::playback_runtime::DawPlaybackRuntime::for_test(tracks, measures),
        log_lines: Arc::new(Mutex::new(VecDeque::new())),
        track_rerender_batches: Arc::new(Mutex::new(vec![None; tracks])),
        solo_tracks: vec![false; tracks],
        track_volumes_db: vec![0; tracks],
        overlays: crate::daw::overlays::DawOverlays::new(1),
        patch_phrase_store: crate::history::PatchPhraseStore::default(),
        patch_phrase_store_dirty: false,

        random_patch_decks: crate::random::RandomIndexDecks::default(),
    }
}

fn render_lines(app: &DawApp, width: u16, height: u16) -> Vec<String> {
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

fn render_buffer(app: &DawApp, width: u16, height: u16) -> Buffer {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(app, f)).unwrap();
    terminal.backend().buffer().clone()
}

fn render_cursor_position(app: &DawApp, width: u16, height: u16) -> Position {
    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).unwrap();
    terminal.draw(|f| draw(app, f)).unwrap();
    terminal.get_cursor_position().unwrap()
}

#[path = "ui/draw_tests.rs"]
mod draw_tests;
#[path = "ui/helpers.rs"]
mod helpers;
#[path = "ui/overlay_tests.rs"]
mod overlay_tests;
#[path = "ui/sound_check_guide.rs"]
mod sound_check_guide;
