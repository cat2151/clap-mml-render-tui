//! `Ctrl+L` で開く演奏設定の描画。
//!
//! 3 項目の checkbox と、下にキー割り当てを 1 行。音色選択より手前（後に描く）ので、
//! 音色を選んでいる最中に開いても隠れない。

use ratatui::{
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    theme::{cursor_highlight_style, MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY},
    ui::centered_rect_with_size,
};

use crate::play_settings::{PlaySettingsItem, PlaySettingsSelect};

const TITLE: &str = " 演奏設定 ";
const HINTS: &str = "↑↓選択  ←→/Space切替  Enter確定  Esc取消";
const WIDTH: u16 = 50;
/// 枠 2 行 + 項目 3 行 + 空行 1 行 + キー割り当て 1 行。
const HEIGHT: u16 = 7;
/// 項目名の桁。補足はこの右へ揃える。項目名はすべて ASCII なので桁＝文字数。
const LABEL_WIDTH: usize = 16;

pub(super) fn draw(select: &PlaySettingsSelect, frame: &mut Frame<'_>) {
    let screen = frame.area();
    let area = centered_rect_with_size(WIDTH.min(screen.width), HEIGHT.min(screen.height), screen);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines(select)).block(block()).style(base()),
        area,
    );
}

fn block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(TITLE)
        .border_style(Style::default().fg(MONOKAI_CYAN))
}

fn base() -> Style {
    Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG)
}

fn lines(select: &PlaySettingsSelect) -> Vec<Line<'static>> {
    let mut lines = PlaySettingsItem::ALL
        .into_iter()
        .enumerate()
        .map(|(index, item)| item_line(select, index, item))
        .collect::<Vec<_>>();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!(" {HINTS}"),
        Style::default().fg(MONOKAI_GRAY),
    )));
    lines
}

fn item_line(select: &PlaySettingsSelect, index: usize, item: PlaySettingsItem) -> Line<'static> {
    let mark = if item.is_on(select.settings()) {
        "[*]"
    } else {
        "[ ]"
    };
    let text = format!(
        " {mark} {label:<width$} {detail}",
        label = item.label(),
        width = LABEL_WIDTH,
        detail = item.detail()
    );
    if index != select.cursor() {
        return Line::from(Span::styled(text, base()));
    }
    // 選択行は枠の内側いっぱいまで塗る。文字の長さぶんだけ色が付くと、
    // 項目名の長さで反転の幅が動いて「どこまでが 1 行か」が読み取れない。
    Line::from(Span::styled(
        pad_to_inner_width(&text),
        cursor_highlight_style(base()),
    ))
}

/// 枠の内側の桁数まで空白で埋める。全角を含むので文字数ではなく表示桁数で数える。
fn pad_to_inner_width(text: &str) -> String {
    let inner = usize::from(WIDTH - 2);
    let padding = inner.saturating_sub(Line::from(text).width());
    format!("{text}{}", " ".repeat(padding))
}
