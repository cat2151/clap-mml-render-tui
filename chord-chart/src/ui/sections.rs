//! 左 pane。素材（section）の定義そのもの。

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{cursor_highlight_style, MONOKAI_GRAY, MONOKAI_YELLOW},
};

use super::{
    pane_block,
    text::{fit_width, right_align},
    visible_range, CURSOR_WIDTH, INDEX_WIDTH, NAME_WIDTH,
};
use crate::{ChordChartScreen, Pane, Section};

/// degrees 以外が固定で食う桁数。degrees は残り全部を貰う。
const FIXED_WIDTH: usize = CURSOR_WIDTH + INDEX_WIDTH + 1 + NAME_WIDTH + 1;

pub(super) fn draw(f: &mut Frame<'_>, area: Rect, screen: &ChordChartScreen) {
    let focused = screen.focus == Pane::Sections;
    let block = pane_block(" Sections ", focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let cursor = screen.clamped_section_cursor();
    let range = visible_range(screen.song.sections.len(), cursor, inner.height as usize);
    let degrees_width = (inner.width as usize).saturating_sub(FIXED_WIDTH);
    let lines: Vec<Line<'static>> = range
        .map(|index| {
            row(
                index,
                &screen.song.sections[index],
                degrees_width,
                index == cursor,
                focused,
            )
        })
        .collect();
    f.render_widget(Paragraph::new(lines).style(base_style()), inner);
}

fn row(
    index: usize,
    section: &Section,
    degrees_width: usize,
    on_cursor: bool,
    focused: bool,
) -> Line<'static> {
    let mut spans = vec![
        Span::styled(
            if on_cursor { ">" } else { " " }.to_string(),
            base_style().fg(MONOKAI_YELLOW),
        ),
        Span::styled(
            right_align(&(index + 1).to_string(), INDEX_WIDTH),
            base_style().fg(MONOKAI_GRAY),
        ),
        Span::styled(" ".to_string(), base_style()),
        Span::styled(fit_width(&section.name, NAME_WIDTH), base_style()),
        Span::styled(" ".to_string(), base_style()),
        // degrees は解釈しないので、良し悪しで色を変えることもしない。
        Span::styled(fit_width(&section.degrees, degrees_width), base_style()),
    ];
    if on_cursor && focused {
        for span in &mut spans {
            span.style = cursor_highlight_style(span.style);
        }
    }
    Line::from(spans)
}
