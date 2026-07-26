//! 主要画面（notepad / DAW / keyboard / loop browser / grid sequencer）の切替メニュー。
//!
//! 画面横断で共有するため `cmrt-tui-core` に置く。

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    layout::Alignment,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::theme::{MONOKAI_CYAN, MONOKAI_FG, MONOKAI_YELLOW};

/// アプリの再開対象になり得る主要画面。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PrimaryScreen {
    #[default]
    Notepad,
    Daw,
    Keyboard,
    LoopBrowser,
    GridSequencer,
}

impl PrimaryScreen {
    const ALL: [(char, Self, &'static str); 5] = [
        ('N', Self::Notepad, "Notepad"),
        ('D', Self::Daw, "DAW"),
        ('K', Self::Keyboard, "Keyboard"),
        ('L', Self::LoopBrowser, "Loop Browser"),
        ('G', Self::GridSequencer, "Grid Sequencer"),
    ];

    fn from_menu_key(key: char) -> Option<Self> {
        Self::ALL.iter().find_map(|(candidate, screen, _)| {
            candidate.eq_ignore_ascii_case(&key).then_some(*screen)
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScreenSwitchMenuAction {
    Continue,
    Closed,
    SwitchTo(PrimaryScreen),
}

#[derive(Default)]
pub struct ScreenSwitchMenu {
    open: bool,
}

impl ScreenSwitchMenu {
    pub fn is_open(&self) -> bool {
        self.open
    }

    pub fn open(&mut self) {
        self.open = true;
    }

    pub fn close(&mut self) {
        self.open = false;
    }

    pub fn handle_key(&mut self, key: KeyEvent) -> ScreenSwitchMenuAction {
        match key.code {
            KeyCode::Esc => {
                self.open = false;
                ScreenSwitchMenuAction::Closed
            }
            KeyCode::Char(character)
                if matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT) =>
            {
                let Some(target) = PrimaryScreen::from_menu_key(character) else {
                    return ScreenSwitchMenuAction::Continue;
                };
                self.open = false;
                ScreenSwitchMenuAction::SwitchTo(target)
            }
            _ => ScreenSwitchMenuAction::Continue,
        }
    }
}

pub fn is_screen_switch_trigger(key: KeyEvent) -> bool {
    key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('g')
}

pub fn draw_screen_switch_menu(frame: &mut Frame<'_>, current: PrimaryScreen) {
    const TITLE: &str = " Screen Switch  Esc:cancel ";
    let lines = PrimaryScreen::ALL
        .iter()
        .map(|(key, screen, label)| {
            let style = if *screen == current {
                Style::default()
                    .fg(MONOKAI_YELLOW)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(MONOKAI_FG)
            };
            Line::from(Span::styled(format!("[{key}] {label}"), style))
        })
        .collect::<Vec<_>>();
    let area = crate::ui::centered_text_block_rect(frame.area(), TITLE, &lines);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Left)
            .style(Style::default().fg(MONOKAI_FG))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(TITLE)
                    .border_style(Style::default().fg(MONOKAI_CYAN)),
            ),
        area,
    );
}

#[cfg(test)]
mod tests;
