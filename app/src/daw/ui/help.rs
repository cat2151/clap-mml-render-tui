use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::{MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY, MONOKAI_YELLOW};

pub(super) fn draw_help(f: &mut Frame, area: Rect) {
    let popup = crate::ui_utils::centered_rect(60, 96, area);
    f.render_widget(Clear, popup);

    let help_lines = vec![
        Line::from(Span::styled(
            "NORMAL モード",
            Style::default()
                .fg(MONOKAI_YELLOW)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  h / ←  : 小節移動（左）"),
        Line::from("  l / →  : 小節移動（右）"),
        Line::from("  j / ↓  : track 移動（下）"),
        Line::from("  k / ↑  : track 移動（上）"),
        Line::from("  H      : 先頭 track へ移動"),
        Line::from("  M      : 中央 track へ移動"),
        Line::from("  L      : 末尾 track へ移動"),
        Line::from("  i      : INSERT モード"),
        Line::from("  a      : off → start固定/end追従 → end固定 → off"),
        Line::from("  m      : mixer overlay"),
        Line::from("  p      : 演奏 / 停止"),
        Line::from("  s      : solo toggle"),
        Line::from("  r      : random 音色設定"),
        Line::from("  K / ?  : ヘルプ (このページ)"),
        Line::from("  d/ESC  : TUI に戻る"),
        Line::from("  q      : 終了"),
        Line::from(""),
        Line::from(Span::styled(
            "INSERT モード",
            Style::default()
                .fg(MONOKAI_YELLOW)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  ESC   : 確定 → NORMAL"),
        Line::from("  Enter : 確定 → 次小節 → INSERT 継続"),
        Line::from("  Ctrl+C/X/V: コピー / カット / ペースト"),
        Line::from(Span::styled(
            "MIXER overlay",
            Style::default()
                .fg(MONOKAI_YELLOW)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  h/l, ←/→ : track 移動"),
        Line::from("  j/k, ↓/↑ : volume -/+3dB"),
        Line::from("  ESC      : 閉じる"),
        Line::from(""),
        Line::from(Span::styled(
            "  [ESC] でキャンセル",
            Style::default().fg(MONOKAI_GRAY),
        )),
    ];

    f.render_widget(
        Paragraph::new(help_lines)
            .style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" ヘルプ (Keybinds) ")
                    .border_style(Style::default().fg(MONOKAI_CYAN)),
            ),
        popup,
    );
}
