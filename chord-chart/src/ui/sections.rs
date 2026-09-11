//! 左 pane。素材（section）の定義そのもの。

use std::ops::Range;

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
    degrees::degrees_spans,
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
    // 反転を描くのは**フォーカスしている pane のカーソル行だけ**。chord カーソルは
    // そこにしか無い（行を移ると先頭へ戻る）。範囲を知っているのは screen の写しだけで、
    // ここで degrees を読んで数えることはしない（ADR 0020）。
    let highlight = focused.then(|| screen.cursor_chord_range()).flatten();
    let lines: Vec<Line<'static>> = range
        .map(|index| {
            row(
                index,
                &screen.song.sections[index],
                degrees_width,
                index == cursor,
                focused,
                if index == cursor {
                    highlight.clone()
                } else {
                    None
                },
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
    highlight: Option<Range<usize>>,
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
    ];
    // degrees は解釈しないので、良し悪しで色を変えることもしない。
    // 色が変わるのは chord カーソルの当たっている 1 つだけ（反転）。
    spans.extend(degrees_spans(
        &section.degrees,
        degrees_width,
        highlight,
        base_style(),
    ));
    if on_cursor && focused {
        for span in &mut spans {
            span.style = cursor_highlight_style(span.style);
        }
    }
    Line::from(spans)
}
