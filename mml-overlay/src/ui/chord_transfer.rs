//! chord ヒント 1 行と、確定ダイアログの描画。
//!
//! ヒントは**立っているときだけ**行が増える。常設すると overlay の高さが 1 行
//! 増えたまま戻らず、DAW の小節セルを 1 行で編集するという手触りが変わる。

use ratatui::{
    layout::Rect,
    style::Style,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    theme::{cursor_highlight_style, MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY},
    ui::centered_rect_with_size,
};

use crate::chord_transfer::{ChordTransferChoice, ChordTransferConfirm, CHORD_HINT};
use crate::MmlOverlay;

const TITLE: &str = " chord として解釈できます ";
const HINTS: &str = "↑↓選択  Enter確定  Esc取消（入力欄へ戻る）";
const WIDTH: u16 = 56;
/// 枠 2 行 + 選択肢 2 行 + 空行 1 行 + キー割り当て 1 行。
const HEIGHT: u16 = 6;
/// 選択肢の桁。補足はこの右へ揃える。全角を含むので表示桁で数える。
const LABEL_WIDTH: usize = 26;

/// ヒントの行。立っていなければ 0 行。
pub(super) fn hint_rows(overlay: &MmlOverlay<'_>) -> u16 {
    u16::from(overlay.chord_hint())
}

pub(super) fn draw_hint(frame: &mut Frame<'_>, area: Rect) {
    frame.render_widget(
        Paragraph::new(format!(" {CHORD_HINT}")).style(Style::default().fg(MONOKAI_GRAY)),
        area,
    );
}

pub(super) fn draw(confirm: &ChordTransferConfirm, frame: &mut Frame<'_>) {
    let screen = frame.area();
    let area = centered_rect_with_size(WIDTH.min(screen.width), HEIGHT.min(screen.height), screen);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines(confirm)).block(block()).style(base()),
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

fn lines(confirm: &ChordTransferConfirm) -> Vec<Line<'static>> {
    let mut lines = ChordTransferChoice::ALL
        .into_iter()
        .enumerate()
        .map(|(index, choice)| choice_line(confirm, index, choice))
        .collect::<Vec<_>>();
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        format!(" {HINTS}"),
        Style::default().fg(MONOKAI_GRAY),
    )));
    lines
}

fn choice_line(
    confirm: &ChordTransferConfirm,
    index: usize,
    choice: ChordTransferChoice,
) -> Line<'static> {
    let label = choice.label();
    let padding = LABEL_WIDTH.saturating_sub(Line::from(label).width());
    let text = format!(
        " {label}{} {detail}",
        " ".repeat(padding),
        detail = choice.detail()
    );
    if index != confirm.cursor() {
        return Line::from(Span::styled(text, base()));
    }
    // 選択行は枠の内側いっぱいまで塗る（演奏設定モーダルと同じ流儀）。
    Line::from(Span::styled(
        pad_to_inner_width(&text),
        cursor_highlight_style(base()),
    ))
}

fn pad_to_inner_width(text: &str) -> String {
    let inner = usize::from(WIDTH - 2);
    let padding = inner.saturating_sub(Line::from(text).width());
    format!("{text}{}", " ".repeat(padding))
}
