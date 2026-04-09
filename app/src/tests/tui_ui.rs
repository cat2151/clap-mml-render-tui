use ratatui::layout::{Constraint, Direction, Layout};
use ratatui::{backend::TestBackend, buffer::Buffer, style::Color, Terminal};

pub(super) use crate::test_utils::{find_text_ignoring_spaces, help_overlay_bounds};
use crate::ui_theme::{
    cursor_highlight_bg, MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY, MONOKAI_GREEN,
    MONOKAI_PURPLE, MONOKAI_YELLOW,
};
use crate::{config::Config, history::PatchPhraseState, tui::TuiApp};

use super::{draw, status_color, Mode, PlayState};

fn test_config() -> Config {
    Config {
        plugin_path: "/tmp/Surge XT.clap".to_string(),
        input_midi: "input.mid".to_string(),
        output_midi: "output.mid".to_string(),
        output_wav: "output.wav".to_string(),
        sample_rate: 44_100.0,
        buffer_size: 512,
        patches_dirs: Some(vec!["/tmp/patches".to_string()]),
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

#[path = "tui_ui/colors_and_footer.rs"]
mod colors_and_footer;
#[path = "tui_ui/help_screens.rs"]
mod help_screens;
#[path = "tui_ui/overlay_screens.rs"]
mod overlay_screens;
