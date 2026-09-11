//! 右 pane。曲の並び。1 行 = arrangement の 1 要素。

use std::ops::Range;

use ratatui::{
    layout::Rect,
    text::{Line, Span},
    widgets::Paragraph,
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{cursor_highlight_style, MONOKAI_GRAY, MONOKAI_PINK, MONOKAI_YELLOW},
};

use super::{
    degrees::degrees_spans,
    pane_block,
    text::{fit_width, right_align},
    visible_range, CURSOR_WIDTH, INDEX_WIDTH, NAME_WIDTH,
};
use crate::{ChordChartScreen, Pane, Song};

/// degrees 以外が固定で食う桁数。
const FIXED_WIDTH: usize = CURSOR_WIDTH + INDEX_WIDTH + 1 + NAME_WIDTH + 1;

pub(super) fn draw(f: &mut Frame<'_>, area: Rect, screen: &ChordChartScreen) {
    let focused = screen.focus == Pane::Arrangement;
    let block = pane_block(" Arrangement ", focused);
    let inner = block.inner(area);
    f.render_widget(block, area);

    let song = &screen.song;
    let cursor = screen.clamped_arrangement_cursor();
    let range = visible_range(song.arrangement.len(), cursor, inner.height as usize);
    let degrees_width = (inner.width as usize).saturating_sub(FIXED_WIDTH);
    // 反転はフォーカスしている pane のカーソル行だけ。右 pane では**参照先**
    // section の degrees を描いているので、範囲もその section のものが来る
    // （`cursor_chord_range` が `preview_target` を通る）。
    let highlight = focused.then(|| screen.cursor_chord_range()).flatten();
    // 行番号と `1`..`9` の挿入位置を一致させるため、`arranged_sections()` ではなく
    // `arrangement` を直接回す（参照先が引けない行も 1 行として出す）。
    let lines: Vec<Line<'static>> = range
        .map(|index| {
            row(
                song,
                index,
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
    song: &Song,
    index: usize,
    degrees_width: usize,
    on_cursor: bool,
    focused: bool,
    highlight: Option<Range<usize>>,
) -> Line<'static> {
    let section = song.arrangement.get(index).and_then(|id| song.section(*id));
    // 進行の良し悪しは見ない（この画面は degrees を解釈しない）。色を変えるのは
    // 「参照先の section が引けない」ときだけ。
    let missing = section.is_none();
    let name = section.map_or("?", |section| section.name.as_str());
    let degrees = section.map_or("(不明な section)", |section| section.degrees.as_str());

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
        Span::styled(fit_width(name, NAME_WIDTH), base_style()),
        Span::styled(" ".to_string(), base_style()),
    ];
    let degrees_style = if missing {
        base_style().fg(MONOKAI_PINK)
    } else {
        base_style()
    };
    // 反転するのは chord カーソルの当たっている 1 つだけ。参照が壊れている行は
    // 参照先が無い＝範囲も来ないので、`(不明な section)` が割れることはない。
    spans.extend(degrees_spans(
        degrees,
        degrees_width,
        highlight,
        degrees_style,
    ));
    if on_cursor && focused {
        for span in &mut spans {
            span.style = cursor_highlight_style(span.style);
        }
    }
    Line::from(spans)
}
