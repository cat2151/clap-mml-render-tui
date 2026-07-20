use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Wrap},
    Frame,
};

use super::status::{base_style, loop_browser_keybind_text, status_color, status_text};
use crate::loop_wav_analysis::format_analysis;
use crate::tui::loop_browser::{LoopBrowserPane, PAD_KEYS};
use crate::tui::{Mode, TuiApp};
use crate::ui_theme::{
    cursor_highlight_style, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GREEN, MONOKAI_YELLOW,
};

const TRACK_LABEL_WIDTH: usize = 6;
const CELL_WIDTH: usize = 14;

pub(super) fn draw(app: &mut TuiApp<'_>, frame: &mut Frame) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(6),
            Constraint::Length(3),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(frame.area());
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(50), Constraint::Percentage(50)])
        .split(chunks[0]);

    draw_tree(app, frame, panes[0]);
    draw_tracks(app, frame, panes[1]);
    draw_pads(app, frame, chunks[1]);

    let play_state = app.play_state.lock().unwrap().clone();
    let persistence_error = app
        .loop_browser
        .metadata_error
        .as_ref()
        .or(app.loop_browser.track_grid_error.as_ref());
    let (status, color) = if let Some(error) = persistence_error {
        (error.clone(), Color::Red)
    } else {
        (
            status_text(&Mode::LoopBrowser, &play_state),
            status_color(&play_state),
        )
    };
    frame.render_widget(
        Paragraph::new(status).style(base_style().fg(color)),
        chunks[2],
    );
    frame.render_widget(
        Paragraph::new(loop_browser_keybind_text(app.loop_browser.focus)).style(base_style()),
        chunks[3],
    );

    if app.loop_browser.category_overlay.is_some() {
        draw_category_overlay(app, frame);
    }
    if let Some(notice) = app
        .loop_browser
        .active_notice()
        .map(|notice| notice.text.clone())
    {
        draw_notice(frame, &notice);
    }
}

fn draw_tree(app: &mut TuiApp<'_>, frame: &mut Frame<'_>, area: Rect) {
    let focused = app.loop_browser.focus == LoopBrowserPane::Tree;
    let border = focus_border_style(focused);
    let title = if app.loop_browser.favorites_only {
        " [LOOP TREE] Favorite dirs "
    } else {
        " [LOOP TREE] WAV loops "
    };
    if let Some(error) = &app.loop_browser.error {
        frame.render_widget(
            Paragraph::new(error.as_str())
                .wrap(Wrap { trim: false })
                .style(base_style().fg(Color::Red))
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(title)
                        .border_style(border),
                ),
            area,
        );
        return;
    }

    let items = app
        .loop_browser
        .visible
        .iter()
        .map(|node| {
            let marker = if node.is_wav {
                "♪ "
            } else if node.expanded {
                "▾ "
            } else {
                "▸ "
            };
            let favorite = if !node.is_wav && node.favorite {
                "★ "
            } else {
                ""
            };
            let category = node
                .category
                .as_ref()
                .map(|category| format!(" [{category}]"))
                .unwrap_or_default();
            ListItem::new(Line::from(vec![
                Span::raw("  ".repeat(node.depth)),
                Span::raw(marker),
                Span::styled(favorite, base_style().fg(MONOKAI_YELLOW)),
                Span::raw(node.name.clone()),
                Span::styled(
                    node.analysis
                        .map(|analysis| format!(" {}", format_analysis(analysis)))
                        .unwrap_or_default(),
                    base_style().fg(MONOKAI_CYAN),
                ),
                Span::styled(category, base_style().fg(MONOKAI_GREEN)),
            ]))
        })
        .collect::<Vec<_>>();
    frame.render_stateful_widget(
        List::new(items)
            .style(base_style())
            .highlight_style(cursor_highlight_style(base_style()))
            .highlight_symbol("▶ ")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(title)
                    .border_style(border),
            ),
        area,
        &mut app.loop_browser.list_state,
    );
}

fn draw_tracks(app: &mut TuiApp<'_>, frame: &mut Frame<'_>, area: Rect) {
    let focused = app.loop_browser.focus == LoopBrowserPane::Tracks;
    let block = Block::default()
        .borders(Borders::ALL)
        .title(" [TRACK LIST BPM120 AUTO-STRETCH] ")
        .border_style(focus_border_style(focused));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.width == 0 || inner.height == 0 {
        return;
    }

    let visible_tracks = usize::from(inner.height.saturating_sub(1)).max(1);
    let visible_measures =
        ((usize::from(inner.width).saturating_sub(TRACK_LABEL_WIDTH)) / CELL_WIDTH).max(1);
    let browser = &mut app.loop_browser;
    keep_visible(
        browser.track_cursor,
        visible_tracks,
        &mut browser.track_scroll,
    );
    keep_visible(
        browser.measure_cursor,
        visible_measures,
        &mut browser.measure_scroll,
    );

    let mut lines = Vec::with_capacity(visible_tracks + 1);
    let mut header = vec![Span::styled(
        fit("track", TRACK_LABEL_WIDTH),
        base_style().fg(MONOKAI_CYAN),
    )];
    for measure in browser.measure_scroll
        ..(browser.measure_scroll + visible_measures).min(browser.track_grid()[0].len())
    {
        let style = if focused && measure == browser.measure_cursor {
            base_style().fg(MONOKAI_YELLOW)
        } else {
            base_style().fg(MONOKAI_CYAN)
        };
        header.push(Span::styled(
            fit(&format!("measure {}", measure + 1), CELL_WIDTH),
            style,
        ));
    }
    lines.push(Line::from(header));

    for track in browser.track_scroll
        ..(browser.track_scroll + visible_tracks).min(browser.track_grid().len())
    {
        let mut spans = vec![Span::styled(
            fit(&format!("T{}", track + 1), TRACK_LABEL_WIDTH),
            base_style().fg(MONOKAI_CYAN),
        )];
        for measure in browser.measure_scroll
            ..(browser.measure_scroll + visible_measures).min(browser.track_grid()[track].len())
        {
            let label = browser
                .clip_at(track, measure)
                .map(|(start, clip)| {
                    if start == measure {
                        browser.cell_label(&clip.wav)
                    } else {
                        format!("↳ {}/{}", measure - start + 1, clip.span_measures)
                    }
                })
                .unwrap_or_else(|| "·".to_string());
            let style =
                if focused && track == browser.track_cursor && measure == browser.measure_cursor {
                    cursor_highlight_style(base_style())
                } else {
                    base_style()
                };
            spans.push(Span::styled(fit(&label, CELL_WIDTH), style));
        }
        lines.push(Line::from(spans));
    }
    frame.render_widget(Paragraph::new(lines).style(base_style()), inner);
}

fn draw_pads(app: &TuiApp<'_>, frame: &mut Frame<'_>, area: Rect) {
    let spans = PAD_KEYS
        .iter()
        .flat_map(|pad| {
            let name = app
                .loop_browser
                .pad_file_name(*pad)
                .unwrap_or_else(|| "-".to_string());
            [
                Span::styled(
                    format!(" {}:", pad.to_ascii_uppercase()),
                    base_style().fg(MONOKAI_YELLOW),
                ),
                Span::raw(format!("{name} ")),
            ]
        })
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(Line::from(spans))
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" [WAV PADS] ")
                    .border_style(base_style().fg(MONOKAI_CYAN)),
            ),
        area,
    );
}

fn focus_border_style(focused: bool) -> Style {
    if focused {
        base_style().fg(MONOKAI_CYAN).add_modifier(Modifier::BOLD)
    } else {
        base_style().fg(MONOKAI_FG)
    }
}

fn keep_visible(cursor: usize, visible: usize, scroll: &mut usize) {
    if cursor < *scroll {
        *scroll = cursor;
    } else if cursor >= *scroll + visible {
        *scroll = cursor + 1 - visible;
    }
}

fn fit(text: &str, width: usize) -> String {
    let mut output = text.chars().take(width).collect::<String>();
    let count = output.chars().count();
    output.extend(std::iter::repeat_n(' ', width.saturating_sub(count)));
    output
}

fn draw_category_overlay(app: &TuiApp<'_>, frame: &mut Frame<'_>) {
    let current = app.loop_browser.category_overlay_current();
    let lines = app
        .loop_browser
        .category_keys
        .iter()
        .map(|(key, category)| {
            let marker = if current == Some(category.as_str()) {
                "●"
            } else {
                " "
            };
            Line::from(format!(" {marker} {key}: {category} "))
        })
        .collect::<Vec<_>>();
    let area = crate::ui_utils::centered_text_block_rect(
        frame.area(),
        " dirカテゴリ (Esc:キャンセル) ",
        &lines,
    );
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" dirカテゴリ (Esc:キャンセル) ")
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        area,
    );
}

fn draw_notice(frame: &mut Frame<'_>, message: &str) {
    let lines = vec![Line::from(Span::styled(
        message.to_string(),
        base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
    ))];
    let area = crate::ui_utils::centered_text_block_rect(frame.area(), " お知らせ ", &lines);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines)
            .alignment(Alignment::Center)
            .style(base_style())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" お知らせ ")
                    .border_style(base_style().fg(MONOKAI_CYAN)),
            ),
        area,
    );
}
