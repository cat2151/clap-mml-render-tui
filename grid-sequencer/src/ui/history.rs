use ratatui::{
    layout::Rect,
    widgets::{Block, Borders, Clear, List, ListItem, ListState},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_FG},
    ui::centered_rect_with_size,
};

use crate::{GridHistoryPreviewStatus, GridSequencerScreen};

const TITLE: &str = " Grid History  j/k,↑/↓:select+play  Space:stop/replay  Enter:Daily DAWへ全置換  Esc/q/Shift+H:close ";

pub(super) fn draw_overlay(frame: &mut Frame<'_>, screen: &GridSequencerScreen) {
    let rows = screen.history_rows();
    let row_count = rows.len();
    let area = overlay_rect(frame.area(), row_count);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(TITLE)
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_CYAN));
    let inner = block.inner(area);
    frame.render_widget(Clear, area);
    frame.render_widget(block, area);

    let preview = preview_status_line(screen);
    let list_area = Rect::new(
        inner.x,
        inner.y,
        inner.width,
        inner.height.saturating_sub(1),
    );
    let preview_area = Rect::new(
        inner.x,
        inner.y + inner.height.saturating_sub(1),
        inner.width,
        1,
    );
    let items = if rows.is_empty() {
        vec![ListItem::new(
            "まだ履歴がありません（周の先頭が発音すると追加されます）",
        )]
    } else {
        rows.into_iter().map(ListItem::new).collect()
    };
    let mut state =
        ListState::default().with_selected((row_count > 0).then_some(screen.history_selected()));
    let list = List::new(items)
        .style(base_style().fg(MONOKAI_FG))
        .highlight_style(cursor_highlight_style(base_style().fg(MONOKAI_FG)))
        .highlight_symbol("▶ ");
    frame.render_stateful_widget(list, list_area, &mut state);
    frame.render_widget(
        ratatui::widgets::Paragraph::new(preview).style(base_style().fg(MONOKAI_CYAN)),
        preview_area,
    );
}

fn preview_status_line(screen: &GridSequencerScreen) -> String {
    let status = match screen.history_preview_status() {
        GridHistoryPreviewStatus::Idle if screen.history_previewing() => "待機中".to_string(),
        GridHistoryPreviewStatus::Idle => "stopped（Spaceで再試聴）".to_string(),
        GridHistoryPreviewStatus::Rendering { completed, total } => {
            format!("rendering {completed}/{total}")
        }
        GridHistoryPreviewStatus::Playing => "playing meas 1".to_string(),
        GridHistoryPreviewStatus::Finished => "finished（Spaceで再試聴）".to_string(),
        GridHistoryPreviewStatus::Error(error) => format!("preview error: {error}"),
    };
    format!(" Preview: {status}")
}

fn overlay_rect(area: Rect, row_count: usize) -> Rect {
    let width = area.width.saturating_sub(4).min(140);
    let height = u16::try_from(row_count.max(1) + 3)
        .unwrap_or(u16::MAX)
        .min(area.height.saturating_sub(2))
        .max(3);
    centered_rect_with_size(width, height, area)
}
