//! ステータス表示の共通ヘルパ（画面横断で共有）。

use ratatui::style::{Color, Style};

use crate::play_state::PlayState;
use crate::theme::{
    MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GREEN, MONOKAI_PURPLE, MONOKAI_YELLOW,
};

pub fn base_style() -> Style {
    Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG)
}

pub fn status_color(play_state: &PlayState) -> Color {
    match play_state {
        PlayState::Err(_) => Color::Red,
        PlayState::Running(_) => MONOKAI_PURPLE,
        PlayState::Playing(_) => MONOKAI_YELLOW,
        PlayState::Done(_) => MONOKAI_GREEN,
        PlayState::Idle => MONOKAI_CYAN,
    }
}

/// 再生状態に応じてステータス行末尾へ付与する文言を返す。
pub fn play_status_suffix(play_state: &PlayState) -> String {
    match play_state {
        PlayState::Idle => "".to_string(),
        PlayState::Running(mml) => format!("  ⚙ レンダリング中: {}", mml),
        PlayState::Playing(msg) => format!("  ▶ 演奏中: {}", msg),
        PlayState::Done(msg) => format!("  ✓ {}", msg),
        PlayState::Err(msg) => format!("  ✗ {}", msg),
    }
}
