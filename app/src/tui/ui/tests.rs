use ratatui::{backend::TestBackend, buffer::Buffer, style::Color, Terminal};

pub(super) use crate::test_utils::{find_text_ignoring_spaces, help_overlay_bounds};
use crate::{config::Config, tui::TuiApp};
use cmrt_tui_core::theme::{
    cursor_highlight_bg, MONOKAI_CYAN, MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_YELLOW,
};

use cmrt_notepad::Mode;

use super::draw;

fn test_config() -> Config {
    Config {
        plugin_path: "/tmp/Surge XT.clap".to_string(),
        input_midi: "input.mid".to_string(),
        output_midi: "output.mid".to_string(),
        output_wav: "output.wav".to_string(),
        sample_rate: 44_100.0,
        buffer_size: 512,
        patches_dirs: Some(vec!["/tmp/patches".to_string()]),
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
        realtime_play_server_prewarm: false,
        autoplay_on_startup: true,
        voicing_shared_source: String::new(),
        voicing_override_source: String::new(),
        chord_progression_source: String::new(),
        ..Default::default()
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

mod cursor_style;
mod help_screens;
mod keyboard_screen;
mod loop_browser;
mod loop_browser_used_wavs;
mod loop_browser_waveform;
mod play_server_notice;
mod render_contract;
mod screen_switch;
