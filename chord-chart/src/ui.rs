//! Chord Chart 画面の描画。
//!
//! 縦に「要約 1 行 / 左右 2 pane / 下段 1 行」。左 pane は素材（section）の定義、
//! 右 pane は曲の並び（arrangement）。help は最後に overlay として重ねる。

use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_GRAY, MONOKAI_PINK, MONOKAI_YELLOW},
    ui::draw_frame_background,
};

use crate::ChordChartScreen;

mod arrangement;
mod degrees;
mod header;
mod help;
mod line_input;
mod sections;
mod text;

#[cfg(test)]
mod tests;

/// 左 pane が使う幅の割合。残りが右 pane。
const SECTIONS_PANE_PERCENT: u16 = 45;
/// カーソル記号 `>` の桁。
const CURSOR_WIDTH: usize = 1;
/// 行番号の桁。
const INDEX_WIDTH: usize = 2;
/// section 名の桁。
const NAME_WIDTH: usize = 8;

/// 描画とテストが同じ矩形を見るための、唯一の layout の作り方。
///
/// 描画側とテスト側で別々に組むと、見えている pane と読んでいる pane がずれる。
pub(crate) struct ChordChartLayout {
    pub header: Rect,
    pub sections: Rect,
    pub arrangement: Rect,
    pub status: Rect,
}

pub(crate) fn layout_for(area: Rect) -> ChordChartLayout {
    let inner = screen_block().inner(area);
    let rows = Layout::vertical([
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ])
    .split(inner);
    let panes = Layout::horizontal([
        Constraint::Percentage(SECTIONS_PANE_PERCENT),
        Constraint::Min(0),
    ])
    .split(rows[1]);
    ChordChartLayout {
        header: rows[0],
        sections: panes[0],
        arrangement: panes[1],
        status: rows[2],
    }
}

fn screen_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(" Chord Chart ")
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_GRAY))
}

pub fn draw(screen: &ChordChartScreen, f: &mut Frame<'_>) {
    draw_frame_background(f);
    let layout = layout_for(f.area());
    f.render_widget(screen_block(), f.area());
    f.render_widget(
        Paragraph::new(header::line(&screen.song)).style(base_style()),
        layout.header,
    );
    sections::draw(f, layout.sections, screen);
    arrangement::draw(f, layout.arrangement, screen);
    f.render_widget(
        Paragraph::new(status_line(screen)).style(base_style()),
        layout.status,
    );
    if screen.help_open {
        help::draw_overlay(f);
    }
    // 入力欄はヘルプより前面。ヘルプが開いている間はキーを食うので同時には開かないが、
    // 順序を決めておかないと将来どちらが上か分からなくなる。
    line_input::draw_overlay(f, screen);
}

/// 下段 1 行。**理由がある間はキー一覧より理由を優先する**
/// （「押したのに何も起きない」を無言で終わらせない）。
fn status_line(screen: &ChordChartScreen) -> Line<'static> {
    match &screen.error {
        Some(error) => Line::from(Span::styled(
            format!(" ! {error}"),
            base_style().fg(MONOKAI_PINK),
        )),
        None => Line::from(Span::styled(
            help::KEYBIND_TEXT,
            base_style().fg(MONOKAI_GRAY),
        )),
    }
}

/// pane の枠。フォーカスされている側だけ枠色を変える。
fn pane_block(title: &'static str, focused: bool) -> Block<'static> {
    let border = if focused {
        base_style().fg(MONOKAI_YELLOW)
    } else {
        base_style().fg(MONOKAI_GRAY)
    };
    Block::default()
        .borders(Borders::ALL)
        .title(title)
        .style(base_style())
        .border_style(border)
}

/// カーソルが必ず見えるようにスクロールした、描く行の範囲。
fn visible_range(len: usize, cursor: usize, height: usize) -> std::ops::Range<usize> {
    if len == 0 || height == 0 {
        return 0..0;
    }
    let first = cursor.saturating_sub(height.saturating_sub(1));
    first..len.min(first + height)
}
