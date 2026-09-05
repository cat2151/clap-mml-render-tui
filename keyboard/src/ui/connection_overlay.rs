//! 「まだ鳴らせない」ことを知らせる中央 overlay（keyboard）。
//!
//! 待ちの本体は play server の起動（CLAP instance 14 本）と、そのあとの
//! 音色ロード。実測で前者だけで 1.7〜6.5 秒あるので、段階と進み具合を
//! DAW と同じ共通ウィジェット（[`cmrt_tui_core::startup_progress`]）で出す。

use ratatui::{
    layout::{Alignment, Rect},
    style::Color,
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use cmrt_tui_core::startup_progress::{
    draw_startup_progress_overlay, StartupStep, StartupStepState,
};
use cmrt_tui_core::status::base_style;
use cmrt_tui_core::theme::MONOKAI_PURPLE;

use crate::{KeyboardConnectionPhase, KeyboardConnectionStatus};

/// 音が鳴るまでの段階。DAW と同じ共通ウィジェットで描く（同じ待ちなので）。
const PLAY_SERVER_STEP: &str = "play server 起動";
const PATCH_LOAD_STEP: &str = "音色ロード";

/// 待っているあいだの中央 overlay。
///
/// `Connecting` と `PatchSetting` は**同じ待ちの前半と後半**なので、
/// 1 つの overlay に 2 段階として並べる。以前はどちらも
/// 「connecting... / patch setting...」の 1 行だけで、どこまで進んだのかも
/// 何秒待っているのかも出ていなかった。
pub(super) fn draw_connection_overlay(
    connection: &KeyboardConnectionStatus,
    f: &mut Frame<'_>,
    keyboard_area: Rect,
) {
    if let Some(steps) = startup_steps(&connection.phase, connection.server_startup) {
        let elapsed = connection
            .stage_started_at
            .map(|started| started.elapsed())
            .unwrap_or_default();
        draw_startup_progress_overlay(f, keyboard_area, &steps, elapsed);
        return;
    }
    let (title, lines, border_color, height) = match &connection.phase {
        KeyboardConnectionPhase::Ready
        | KeyboardConnectionPhase::Connecting
        | KeyboardConnectionPhase::PatchSetting => return,
        // まだ何も要求していない状態。段階を並べても全部 Waiting になるだけなので、
        // 「鳴らない理由」だけを 1 行で出す従来のままにする。
        KeyboardConnectionPhase::Idle => (
            " server connection ",
            vec![
                Line::from("connecting..."),
                Line::from("notes unavailable until ready"),
            ],
            MONOKAI_PURPLE,
            5,
        ),
        KeyboardConnectionPhase::Error(error) => (
            " server connection error ",
            vec![
                Line::from(format!("server error: {error}")),
                Line::from("r:retry  n:notepad  w:DAW  q:quit"),
            ],
            Color::Red,
            7,
        ),
    };
    let width = keyboard_area.width.saturating_sub(2).min(72);
    let height = height.min(keyboard_area.height);
    let area = cmrt_tui_core::ui::centered_rect_with_size(width, height, keyboard_area);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .wrap(Wrap { trim: true })
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .style(base_style())
                    .border_style(base_style().fg(border_color)),
            ),
        area,
    );
}

/// 待ちの段階を共通ウィジェットの行へ翻訳する。待っていなければ `None`。
///
/// `Idle` を待ち扱いにしないこと。あれは「まだ何も要求していない」であって、
/// 音が鳴るまでの途中ではない。
fn startup_steps(
    phase: &KeyboardConnectionPhase,
    server_startup: Option<(usize, usize)>,
) -> Option<Vec<StartupStep>> {
    match phase {
        KeyboardConnectionPhase::Connecting => Some(vec![
            StartupStep::new(PLAY_SERVER_STEP, StartupStepState::Running(server_startup)),
            StartupStep::new(PATCH_LOAD_STEP, StartupStepState::Waiting),
        ]),
        KeyboardConnectionPhase::PatchSetting => Some(vec![
            StartupStep::new(PLAY_SERVER_STEP, StartupStepState::Done),
            // patch のロードは 1 instance ぶんなので、数えられる件数が無い。
            StartupStep::new(PATCH_LOAD_STEP, StartupStepState::Running(None)),
        ]),
        KeyboardConnectionPhase::Idle
        | KeyboardConnectionPhase::Ready
        | KeyboardConnectionPhase::Error(_) => None,
    }
}

#[cfg(test)]
mod tests;
