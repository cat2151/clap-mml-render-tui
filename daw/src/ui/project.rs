use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, FrameExt as _, Paragraph, Wrap},
    Frame,
};

use super::{
    super::{DawApp, DawProjectFileAction},
    MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY, MONOKAI_GREEN, MONOKAI_PINK,
    MONOKAI_YELLOW,
};
use crate::messages::project as message;

pub(super) fn draw_project(frame: &mut Frame<'_>, app: &DawApp, area: Rect) {
    let open_action = app
        .overlays
        .project
        .action
        .is_some_and(DawProjectFileAction::is_open);
    let popup_height = if open_action { 72 } else { 38 };
    let popup_width = if open_action { 92 } else { 78 };
    let popup = cmrt_tui_core::ui::centered_rect(popup_width, popup_height, area);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(message::OVERLAY_TITLE)
        .border_style(Style::default().fg(MONOKAI_CYAN))
        .style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    if let Some(action) = app.overlays.project.action {
        match action {
            DawProjectFileAction::SaveAs => draw_path_input(frame, app, inner),
            DawProjectFileAction::Open | DawProjectFileAction::OpenDailyArchive => {
                draw_open_selector(frame, app, inner)
            }
        }
    } else {
        draw_action_menu(frame, app, inner);
    }

    if let Some(backup_path) = &app.overlays.project.backup_notice_path {
        draw_backup_notice(frame, backup_path, inner);
    }
}

fn draw_backup_notice(frame: &mut Frame<'_>, backup_path: &std::path::Path, area: Rect) {
    let width = area.width.saturating_sub(2).min(72);
    let height = area.height.min(5);
    if width < 4 || height < 3 {
        return;
    }
    let notice = Rect::new(
        area.x + area.width.saturating_sub(width) / 2,
        area.y + area.height.saturating_sub(height),
        width,
        height,
    );
    frame.render_widget(Clear, notice);
    frame.render_widget(
        Paragraph::new(vec![
            Line::from(message::BACKUP_RENAMED),
            Line::from(backup_path.display().to_string()),
        ])
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(message::BACKUP_CREATED_TITLE)
                .border_style(Style::default().fg(MONOKAI_GREEN)),
        )
        .style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG))
        .wrap(Wrap { trim: false }),
        notice,
    );
}

fn draw_action_menu(frame: &mut Frame<'_>, app: &DawApp, area: Rect) {
    let current_path = app
        .overlays
        .project
        .current_path
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| message::CURRENT_PATH_UNSET.to_string());
    let lines = vec![
        Line::from(Span::styled(
            message::PROJECT_FILE_DESCRIPTION,
            Style::default().fg(MONOKAI_GRAY),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                "  a",
                Style::default()
                    .fg(MONOKAI_YELLOW)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(message::SAVE_AS_ACTION),
        ]),
        Line::from(vec![
            Span::styled(
                "  o",
                Style::default()
                    .fg(MONOKAI_YELLOW)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(message::OPEN_ACTION),
        ]),
        Line::from(vec![
            Span::styled(
                "  d",
                Style::default()
                    .fg(MONOKAI_YELLOW)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(message::OPEN_DAILY_ARCHIVE_ACTION),
        ]),
        Line::from(vec![
            Span::styled(
                "  ESC",
                Style::default()
                    .fg(MONOKAI_YELLOW)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(message::CLOSE_ACTION),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(
                message::CURRENT_PATH_LABEL,
                Style::default().fg(MONOKAI_GRAY),
            ),
            Span::raw(current_path),
        ]),
    ];
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn draw_path_input(frame: &mut Frame<'_>, app: &DawApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(3),
            Constraint::Min(1),
            Constraint::Length(1),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(message::SAVE_AS_PATH_DESCRIPTION).style(Style::default().fg(MONOKAI_GRAY)),
        chunks[0],
    );

    let path_widget = cmrt_tui_core::text_input::build_query_textarea_widget(
        &app.overlays.project.path_textarea,
        &cmrt_tui_core::text_input::textarea_value(&app.overlays.project.path_textarea),
        message::SAVE_AS_PATH_TITLE,
        message::SAVE_AS_PATH_PLACEHOLDER,
        MONOKAI_CYAN,
    );
    frame.render_widget(&path_widget, chunks[1]);
    frame.set_cursor_position(
        cmrt_tui_core::text_input::single_line_textarea_cursor_position(
            chunks[1],
            &app.overlays.project.path_textarea,
        ),
    );

    if let Some(error) = &app.overlays.project.error {
        frame.render_widget(
            Paragraph::new(error.as_str())
                .style(Style::default().fg(MONOKAI_PINK))
                .wrap(Wrap { trim: false }),
            chunks[2],
        );
    }
    frame.render_widget(
        Paragraph::new(message::SAVE_AS_FOOTER).style(Style::default().fg(MONOKAI_YELLOW)),
        chunks[3],
    );
}

fn draw_open_selector(frame: &mut Frame<'_>, app: &DawApp, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Min(4),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(area);
    frame.render_widget(
        Paragraph::new(message::OPEN_DESCRIPTION).style(Style::default().fg(MONOKAI_GRAY)),
        chunks[0],
    );
    let query_title = if app.overlays.project.filter_active {
        message::FILTER_ACTIVE_TITLE
    } else {
        message::FILTER_TITLE
    };
    let query_widget = cmrt_tui_core::text_input::build_query_textarea_widget(
        &app.overlays.project.query_textarea,
        &app.overlays.project.query,
        query_title,
        message::FILTER_PLACEHOLDER,
        MONOKAI_CYAN,
    );
    frame.render_widget(&query_widget, chunks[1]);
    if app.overlays.project.filter_active {
        frame.set_cursor_position(
            cmrt_tui_core::text_input::single_line_textarea_cursor_position(
                chunks[1],
                &query_widget,
            ),
        );
    }
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(64), Constraint::Percentage(36)])
        .split(chunks[2]);
    if let Some(explorer) = &app.overlays.project.file_explorer {
        frame.render_widget_ref(explorer.widget(), panes[0]);
    } else {
        frame.render_widget(
            Paragraph::new(message::FILE_SELECTOR_UNAVAILABLE)
                .style(Style::default().fg(MONOKAI_PINK)),
            panes[0],
        );
    }
    draw_project_preview(frame, app, panes[1]);
    if let Some(error) = &app.overlays.project.error {
        frame.render_widget(
            Paragraph::new(error.as_str())
                .style(Style::default().fg(MONOKAI_PINK))
                .wrap(Wrap { trim: false }),
            chunks[3],
        );
    }
    frame.render_widget(
        Paragraph::new(if app.overlays.project.filter_active {
            message::FILTER_ACTIVE_FOOTER
        } else {
            message::OPEN_FOOTER
        })
        .style(Style::default().fg(MONOKAI_YELLOW)),
        chunks[4],
    );
}

fn draw_project_preview(frame: &mut Frame<'_>, app: &DawApp, area: Rect) {
    let mode = if app.overlays.project.auto_preview {
        message::PREVIEW_MODE_AUTO
    } else {
        message::PREVIEW_MODE_MANUAL
    };
    let selected = app
        .overlays
        .project
        .selected_path()
        .and_then(|path| {
            path.file_name()
                .map(|name| name.to_string_lossy().into_owned())
        })
        .unwrap_or_else(|| message::NO_SELECTION.to_string());
    let mut lines = vec![
        Line::from(vec![
            Span::styled(
                message::PREVIEW_MODE_LABEL,
                Style::default().fg(MONOKAI_GRAY),
            ),
            Span::styled(mode, Style::default().fg(MONOKAI_CYAN)),
        ]),
        Line::from(""),
        Line::from(Span::styled(selected, Style::default().fg(MONOKAI_FG))),
        Line::from(""),
    ];
    if let Some(info) = &app.overlays.project.preview_info {
        lines.extend(
            info.lines()
                .map(|line| Line::from(Span::raw(line.to_string()))),
        );
    } else if app.overlays.project.auto_preview {
        lines.push(Line::from(message::AUTO_PREVIEW_GUIDE));
    } else {
        lines.push(Line::from(message::MANUAL_PREVIEW_GUIDE));
    }
    if let Some(error) = &app.overlays.project.preview_error {
        lines.push(Line::from(""));
        lines.extend(error.lines().map(|line| {
            Line::from(Span::styled(
                line.to_string(),
                Style::default().fg(MONOKAI_PINK),
            ))
        }));
    }
    frame.render_widget(
        Paragraph::new(lines)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(message::preview_title(mode)),
            )
            .wrap(Wrap { trim: false }),
        area,
    );
}
