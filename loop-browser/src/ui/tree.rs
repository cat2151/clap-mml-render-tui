use std::ops::Range;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::Color,
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use super::focus_border_style;
use crate::{LoopBrowser, LoopBrowserPane, VisibleLoopNode};
use cmrt_tui_core::status::base_style;
use cmrt_tui_core::theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_GREEN, MONOKAI_YELLOW};

pub fn draw(state: &mut LoopBrowser, frame: &mut Frame<'_>, area: Rect) -> usize {
    let focused = state.focus == LoopBrowserPane::Tree;
    let border = focus_border_style(focused);
    let title = if state.favorites_only {
        " [LOOP TREE] Favorite dirs "
    } else {
        " [LOOP TREE] WAV loops "
    };
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title)
        .border_style(border);
    if let Some(error) = &state.error {
        frame.render_widget(
            Paragraph::new(error.as_str())
                .wrap(Wrap { trim: false })
                .style(base_style().fg(Color::Red))
                .block(block),
            area,
        );
        return 0;
    }

    let inner = block.inner(area);
    frame.render_widget(block, area);
    if inner.height == 0 || inner.width == 0 {
        return 0;
    }
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(0)])
        .split(inner);
    let breadcrumb_segments = state.selected_breadcrumb();
    let category = state.selected_direct_category();
    let (breadcrumb, category) =
        format_breadcrumb_with_category(&breadcrumb_segments, category, rows[0].width);
    frame.render_widget(
        Paragraph::new(Line::from(vec![
            Span::styled(breadcrumb, base_style().fg(MONOKAI_CYAN)),
            Span::styled(category, base_style().fg(MONOKAI_GREEN)),
        ])),
        rows[0],
    );

    let viewport_height = usize::from(rows[1].height);
    if viewport_height == 0 || state.visible.is_empty() {
        return 0;
    }
    let range = visible_range(
        state.cursor,
        state.visible.len(),
        viewport_height,
        &mut state.tree_scroll,
    );
    let items = state.visible[range.clone()]
        .iter()
        .map(tree_item)
        .collect::<Vec<_>>();
    let mut list_state = ListState::default();
    list_state.select(Some(state.cursor - range.start));
    frame.render_stateful_widget(
        List::new(items)
            .style(base_style())
            .highlight_style(cursor_highlight_style(base_style()))
            .highlight_symbol("▶ "),
        rows[1],
        &mut list_state,
    );
    range.len()
}

fn tree_item(node: &VisibleLoopNode) -> ListItem<'_> {
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
    let analysis = node
        .analysis
        .map(|analysis| {
            format!(
                " {}",
                cmrt_loop_domain::loop_wav_analysis::format_analysis(analysis)
            )
        })
        .unwrap_or_default();
    let category = node
        .category
        .as_ref()
        .map(|category| format!(" [{category}]"))
        .unwrap_or_default();
    ListItem::new(Line::from(vec![
        Span::raw("  ".repeat(node.depth)),
        Span::raw(marker),
        Span::styled(favorite, base_style().fg(MONOKAI_YELLOW)),
        Span::raw(node.name.as_str()),
        Span::styled(analysis, base_style().fg(MONOKAI_CYAN)),
        Span::styled(category, base_style().fg(MONOKAI_GREEN)),
    ]))
}

pub fn visible_range(
    cursor: usize,
    total: usize,
    viewport_height: usize,
    scroll: &mut usize,
) -> Range<usize> {
    if total == 0 || viewport_height == 0 {
        *scroll = 0;
        return 0..0;
    }
    let height = viewport_height.min(total);
    let max_scroll = total.saturating_sub(height);
    *scroll = (*scroll).min(max_scroll);
    let margin = if height >= 4 { (height / 4).max(1) } else { 0 };
    if cursor < scroll.saturating_add(margin) {
        *scroll = cursor.saturating_sub(margin).min(max_scroll);
    } else {
        let lower_margin_start = scroll.saturating_add(height.saturating_sub(margin));
        if cursor >= lower_margin_start {
            *scroll = cursor
                .saturating_add(margin)
                .saturating_add(1)
                .saturating_sub(height)
                .min(max_scroll);
        }
    }
    *scroll..scroll.saturating_add(height).min(total)
}

pub fn format_breadcrumb(segments: &[String], width: u16) -> String {
    let width = usize::from(width);
    if width == 0 || segments.is_empty() {
        return String::new();
    }
    let full = segments.join(" › ");
    if text_width(&full) <= width {
        return full;
    }
    let prefix = "… › ";
    let prefix_width = text_width(prefix);
    if width <= prefix_width {
        return "…".chars().take(width).collect();
    }
    let mut suffix = segments.last().cloned().unwrap_or_default();
    if prefix_width + text_width(&suffix) > width {
        let mut tail = String::new();
        for character in suffix.chars().rev() {
            let candidate = format!("{character}{tail}");
            if text_width(&candidate).saturating_add(1) > width {
                break;
            }
            tail = candidate;
        }
        return format!("…{tail}");
    }
    for segment in segments[..segments.len() - 1].iter().rev() {
        let candidate = format!("{segment} › {suffix}");
        if prefix_width + text_width(&candidate) > width {
            break;
        }
        suffix = candidate;
    }
    format!("{prefix}{suffix}")
}

fn format_breadcrumb_with_category(
    segments: &[String],
    category: Option<&str>,
    width: u16,
) -> (String, String) {
    let width = usize::from(width);
    let Some(category) = category else {
        return (format_breadcrumb(segments, width as u16), String::new());
    };
    let category = format!(" [{category}]");
    let category_width = text_width(&category);
    if category_width >= width {
        return (String::new(), truncate_to_width(&category, width));
    }
    (
        format_breadcrumb(segments, (width - category_width) as u16),
        category,
    )
}

fn truncate_to_width(text: &str, width: usize) -> String {
    let mut output = String::new();
    for character in text.chars() {
        let candidate = format!("{output}{character}");
        if text_width(&candidate) > width {
            break;
        }
        output.push(character);
    }
    output
}

fn text_width(text: &str) -> usize {
    Line::from(text).width()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scrolling_keeps_cursor_inside_quarter_margins() {
        let mut scroll = 0;
        assert_eq!(visible_range(7, 100, 12, &mut scroll), 0..12);
        assert_eq!(visible_range(9, 100, 12, &mut scroll), 1..13);
        assert_eq!(visible_range(20, 100, 12, &mut scroll), 12..24);
        assert_eq!(visible_range(13, 100, 12, &mut scroll), 10..22);
    }

    #[test]
    fn scrolling_clamps_at_both_ends_without_blank_rows() {
        let mut scroll = 40;
        assert_eq!(visible_range(0, 50, 12, &mut scroll), 0..12);
        assert_eq!(visible_range(49, 50, 12, &mut scroll), 38..50);
    }

    #[test]
    fn breadcrumb_keeps_the_deepest_segments_when_narrow() {
        let segments = ["loops", "Drums", "Kicks", "Acoustic"].map(str::to_string);
        assert_eq!(
            format_breadcrumb(&segments, 80),
            "loops › Drums › Kicks › Acoustic"
        );
        assert_eq!(format_breadcrumb(&segments, 20), "… › Kicks › Acoustic");
        assert_eq!(format_breadcrumb(&segments, 8), "…coustic");
        assert_eq!(format_breadcrumb(&["ループ".to_string()], 5), "…ープ");
    }

    #[test]
    fn breadcrumb_reserves_space_for_the_direct_category() {
        let segments = ["loops", "Drums", "Acoustic"].map(str::to_string);
        assert_eq!(
            format_breadcrumb_with_category(&segments, Some("drum"), 24),
            ("… › Acoustic".to_string(), " [drum]".to_string())
        );
        assert_eq!(
            format_breadcrumb_with_category(&segments, Some("ドラム"), 7),
            (String::new(), " [ドラ".to_string())
        );
    }
}
