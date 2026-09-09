//! 曲の頭に置く chord2mml の指定（prefix）を、そのまま出す 1 行。

use ratatui::text::{Line, Span};

use cmrt_tui_core::{status::base_style, theme::MONOKAI_CYAN};

use crate::Song;

/// [`Song::prefix`] を**解釈せずそのまま**出す。ラベルも付けない
/// （`Key=` / `BPM` は文字列自身が持っているので、足すと二重に読める）。
pub(super) fn line(song: &Song) -> Line<'static> {
    Line::from(Span::styled(
        format!(" {}", song.prefix),
        base_style().fg(MONOKAI_CYAN),
    ))
}
