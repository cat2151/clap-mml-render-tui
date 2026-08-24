//! MML オーバーレイから開く音色選択の描画。

use ratatui::{
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::Style,
    text::Line,
    widgets::{Block, Borders, Cell, Clear, Paragraph, Row, Table, TableState, Wrap},
    Frame,
};

use cmrt_tui_core::{
    status::{base_style, LIST_HIGHLIGHT_SYMBOL},
    text_input::{
        build_query_textarea_widget, single_line_textarea_cursor_position, textarea_value,
    },
    theme::{cursor_highlight_style, MONOKAI_FG, MONOKAI_PINK, MONOKAI_YELLOW},
    ui::centered_rect,
};

use crate::patch_select::{PatchSelect, PatchSelectFocus};

const QUERY_TITLE: &str = " Regex (空白=AND)  Enter:決定  Esc:取消 ";
const QUERY_PLACEHOLDER: &str = r"例: warm pad|strings";
/// 絞り込み欄の高さ（枠2行 + 入力1行）。
const QUERY_HEIGHT: u16 = 3;
const CATEGORY_COLUMN_WIDTH: u16 = 12;
const LOAD_COLUMN_WIDTH: u16 = 7;

pub(super) fn draw(select: &PatchSelect<'_>, frame: &mut Frame<'_>) {
    let area = centered_rect(94, 78, frame.area());
    frame.render_widget(Clear, area);

    // 案内が無いときは 1 行も取らない。ふだんの見え方を変えないため。
    let notes_height = notes_height(select, area.width);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(QUERY_HEIGHT),
            Constraint::Min(1),
            Constraint::Length(notes_height),
        ])
        .split(area);
    draw_query(select, frame, chunks[0]);
    draw_panes(select, frame, chunks[1]);
    if notes_height > 0 {
        draw_notes(select, frame, chunks[2]);
    }
}

fn draw_panes(select: &PatchSelect<'_>, frame: &mut Frame<'_>, area: Rect) {
    let group_width = (area.width / 5).clamp(12, 22);
    let preset_width = (area.width / 4).clamp(14, 30);
    let panes = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Length(group_width),
            Constraint::Length(preset_width),
            Constraint::Min(12),
        ])
        .split(area);
    draw_groups(select, frame, panes[0]);
    draw_presets(select, frame, panes[1]);
    draw_list(select, frame, panes[2]);
}

/// 案内に要る行数。折り返しぶんも数える（1 行に収まらないと尻切れになる）。
fn notes_height(select: &PatchSelect<'_>, width: u16) -> u16 {
    let notes = select.catalog_notes();
    if notes.is_empty() {
        return 0;
    }
    let width = usize::from(width.max(1));
    notes
        .iter()
        .map(|note| note.chars().count().div_ceil(width).max(1) as u16)
        .sum()
}

/// 「一覧に出ていない音色がある」ことの案内。
///
/// 一覧の中身をいくら眺めても分からない情報なので、枠の外に出す。文言は
/// `cmrt_runtime::SkippedCatalogPlugin::notice_line` が単一ソースで、
/// `cmrt patch-roles` の欄・log.txt の `patch-load: event=skipped` と同じ 1 行。
fn draw_notes(select: &PatchSelect<'_>, frame: &mut Frame<'_>, area: Rect) {
    let lines: Vec<Line<'_>> = select
        .catalog_notes()
        .iter()
        .map(|note| Line::from(note.as_str()))
        .collect();
    frame.render_widget(
        Paragraph::new(lines)
            .style(base_style().fg(MONOKAI_PINK))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn draw_query(select: &PatchSelect<'_>, frame: &mut Frame<'_>, area: Rect) {
    let textarea = select.query_textarea();
    let value = textarea_value(textarea);
    frame.render_widget(
        &build_query_textarea_widget(
            textarea,
            &value,
            QUERY_TITLE,
            QUERY_PLACEHOLDER,
            MONOKAI_YELLOW,
        ),
        area,
    );
    // 音色選択が開いている間、端末のカーソルは絞り込み欄にある。
    // MML 入力欄より後に描くので、こちらの指定が残る。
    frame.set_cursor_position(single_line_textarea_cursor_position(area, textarea));
}

fn pane_block(title: String, focused: bool) -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(base_style())
        .border_style(base_style().fg(if focused { MONOKAI_YELLOW } else { MONOKAI_FG }))
}

fn draw_groups(select: &PatchSelect<'_>, frame: &mut Frame<'_>, area: Rect) {
    let block = pane_block(
        " Role  ←/→ ".to_string(),
        select.focus() == PatchSelectFocus::Groups,
    );
    let rows = select
        .groups()
        .iter()
        .map(|group| Row::new([Cell::from(group.label())]))
        .collect::<Vec<_>>();
    let mut state = TableState::default().with_selected(Some(select.group_cursor()));
    frame.render_stateful_widget(
        Table::new(rows, [Constraint::Fill(1)])
            .block(block)
            .row_highlight_style(cursor_highlight_style(Style::default().fg(MONOKAI_FG)))
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL),
        area,
        &mut state,
    );
}

fn draw_presets(select: &PatchSelect<'_>, frame: &mut Frame<'_>, area: Rect) {
    let block = pane_block(
        " Preset  Ctrl+A:add ".to_string(),
        select.focus() == PatchSelectFocus::Presets,
    );
    let rows = select
        .presets()
        .iter()
        .map(|preset| {
            let prefix = if preset.is_user { "+ " } else { "" };
            Row::new([Cell::from(format!("{prefix}{}", preset.label))])
        })
        .collect::<Vec<_>>();
    let mut state = TableState::default().with_selected(Some(select.preset_cursor()));
    frame.render_stateful_widget(
        Table::new(rows, [Constraint::Fill(1)])
            .block(block)
            .row_highlight_style(cursor_highlight_style(Style::default().fg(MONOKAI_FG)))
            .highlight_symbol(LIST_HIGHLIGHT_SYMBOL),
        area,
        &mut state,
    );
}

fn draw_list(select: &PatchSelect<'_>, frame: &mut Frame<'_>, area: Rect) {
    let block = pane_block(
        list_title(select),
        select.focus() == PatchSelectFocus::Patches,
    );
    let rows = select
        .filtered()
        .map(|patch| {
            Row::new([
                Cell::from(patch.selector_category().unwrap_or("")),
                Cell::from(patch.display()),
                Cell::from(
                    Line::from(load_label(select, patch.display())).alignment(Alignment::Right),
                ),
            ])
        })
        .collect::<Vec<_>>();
    let mut state = TableState::default();
    state.select((!rows.is_empty()).then_some(select.cursor()));
    frame.render_stateful_widget(
        Table::new(
            rows,
            [
                Constraint::Length(CATEGORY_COLUMN_WIDTH),
                Constraint::Fill(1),
                Constraint::Length(LOAD_COLUMN_WIDTH),
            ],
        )
        .header(Row::new([
            Cell::from("Category"),
            Cell::from("Patch"),
            Cell::from(Line::from("Load").alignment(Alignment::Right)),
        ]))
        .block(block)
        .row_highlight_style(cursor_highlight_style(Style::default().fg(MONOKAI_FG)))
        .highlight_symbol(LIST_HIGHLIGHT_SYMBOL),
        area,
        &mut state,
    );
}

fn load_label(select: &PatchSelect<'_>, patch: &str) -> String {
    select
        .load_measurement(patch)
        .and_then(|measurement| measurement.second_load_ms)
        .map_or_else(|| "-".to_string(), format_load_time)
}

fn format_load_time(milliseconds: u64) -> String {
    match milliseconds {
        0..=99 => format!("{milliseconds}ms"),
        100..=999 => format!("0.{}s", milliseconds / 100),
        _ => format!("{}s", milliseconds / 1_000),
    }
}

fn list_title(select: &PatchSelect<'_>) -> String {
    if select.filter_error().is_some() {
        return " Regex error  Ctrl+R:random ".to_string();
    }
    format!(
        " 音色 ({}/{}) Ctrl+R:random ",
        select.filtered_len(),
        select.total()
    )
}

#[cfg(test)]
mod tests;
