use std::ops::Range;

use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Wrap},
    Frame,
};

use super::focus_border_style;
use crate::{LoopBrowser, LoopBrowserPane, VisibleLoopNode};
use cmrt_tui_core::status::base_style;
use cmrt_tui_core::text_filter::is_valid_condition;
use cmrt_tui_core::text_input::{
    build_query_textarea_widget, single_line_textarea_cursor_position, textarea_value,
};
use cmrt_tui_core::theme::{cursor_highlight_style, MONOKAI_CYAN, MONOKAI_GREEN, MONOKAI_YELLOW};

/// `/` の絞り込み入力欄の高さ（枠2行 + 入力1行）。入力中だけツリーから縦を借りる。
const FILTER_INPUT_HEIGHT: u16 = 3;
const FILTER_INPUT_TITLE: &str = " 絞り込み Enter:確定 Esc:取消 ";
/// 条件が正規表現として壊れているときに、枠の色と一緒に出す文言。
const INVALID_CONDITION: &str = " 不正な条件 ";
const FILTER_PLACEHOLDER: &str = "空白区切りAND・正規表現・大小無視";
const NO_MATCH: &str = "該当なし";

pub fn draw(state: &mut LoopBrowser, frame: &mut Frame<'_>, area: Rect) -> usize {
    let focused = state.focus == LoopBrowserPane::Tree;
    let border = focus_border_style(focused);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(tree_title(state, area.width))
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
    let input_height = if state.filter_input_active() {
        FILTER_INPUT_HEIGHT
    } else {
        0
    };
    let rows = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1),
            Constraint::Length(input_height),
            Constraint::Min(0),
        ])
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
    draw_filter_input(state, frame, rows[1]);

    let list_area = rows[2];
    let viewport_height = usize::from(list_area.height);
    if viewport_height == 0 {
        return 0;
    }
    if state.visible.is_empty() {
        if filter_summary(state).is_some() {
            frame.render_widget(
                Paragraph::new(NO_MATCH).style(base_style().fg(MONOKAI_YELLOW)),
                list_area,
            );
        }
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
            .highlight_style(list_highlight_style(state))
            .highlight_symbol("▶ "),
        list_area,
        &mut list_state,
    );
    range.len()
}

/// 現在行の強調。絞り込み入力中は bg 強調も BOLD も落とし、`▶ ` の行頭記号だけ残す。
///
/// 入力中に見えるカーソルを入力欄の 1 つだけにするため（issue #334）。
/// 右ペイン（tracks / used wavs / waveform）の強調は `focus == Tracks` が条件で、
/// 絞り込み入力は tree に focus があるときしか開けないので、こちらは元から消えている。
fn list_highlight_style(state: &LoopBrowser) -> Style {
    if state.filter_input_active() {
        base_style()
    } else {
        cursor_highlight_style(base_style())
    }
}

/// ペインのタイトル。絞り込み中は確定後もクエリとヒット件数を出し続ける
/// （一覧が減っている理由が画面から分かるように）。
///
/// ツリーのペインは画面幅の 40% しかないので、そのままでは件数が枠の外へ出て消える。
/// 入りきらないときは、消しても意味が変わらないもの（種別ラベル → 画面名）から順に落とす。
fn tree_title(state: &LoopBrowser, width: u16) -> String {
    let base = if state.favorites_only {
        "Favorite dirs"
    } else {
        "WAV loops"
    };
    let Some(summary) = filter_summary(state) else {
        return format!(" [LOOP TREE] {base} ");
    };
    // 枠線の左右 2 桁を除いた、タイトルに使える幅。
    let width = usize::from(width).saturating_sub(2);
    let candidates = [
        format!(" [LOOP TREE] {base}  {summary} "),
        format!(" [LOOP TREE] {summary} "),
        format!(" {summary} "),
    ];
    for candidate in &candidates {
        if text_width(candidate) <= width {
            return candidate.clone();
        }
    }
    truncate_to_width(&format!(" {summary} "), width)
}

/// 絞り込み中なら `filter: kick (12 wav)`。絞り込んでいなければ `None`。
///
/// 絞り込み中は残ったディレクトリが全部展開されているので、可視の wav 行数が
/// そのままヒットした wav の本数になる。
fn filter_summary(state: &LoopBrowser) -> Option<String> {
    let query = state.filter_query();
    if query.is_empty() && !state.filter_active() {
        return None;
    }
    let hits = state.visible.iter().filter(|node| node.is_wav).count();
    let invalid = if is_valid_condition(query) {
        ""
    } else {
        INVALID_CONDITION
    };
    Some(format!("filter: {query} ({hits} wav){invalid}"))
}

/// `/` の入力中だけ、breadcrumb の下に枠つきの 1 行入力欄を出す。確定したら消す。
///
/// 条件が正規表現として壊れているときは、直前の有効な結果を消さずに枠だけ赤くする
/// （打鍵の途中の `(` や `[` で一覧が消えないように）。
fn draw_filter_input(state: &LoopBrowser, frame: &mut Frame<'_>, area: Rect) {
    let Some(textarea) = state.filter_textarea() else {
        return;
    };
    if area.width == 0 || area.height == 0 {
        return;
    }
    let value = textarea_value(textarea);
    let (title, border) = if is_valid_condition(&value) {
        (FILTER_INPUT_TITLE.to_string(), MONOKAI_YELLOW)
    } else {
        (
            format!("{FILTER_INPUT_TITLE}{INVALID_CONDITION}"),
            Color::Red,
        )
    };
    frame.render_widget(
        &build_query_textarea_widget(textarea, &value, title, FILTER_PLACEHOLDER, border),
        area,
    );
    frame.set_cursor_position(single_line_textarea_cursor_position(area, textarea));
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
mod tests;
