use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

pub(super) fn draw_help(f: &mut Frame, area: Rect) {
    let popup = crate::ui_utils::centered_rect(60, 80, area);
    f.render_widget(Clear, popup);

    let help_lines = vec![
        Line::from(Span::styled(
            "NORMAL モード",
            Style::default()
                .fg(Color::Yellow)
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
        Line::from("  p      : 演奏 / 停止"),
        Line::from("  r      : random 音色設定"),
        Line::from("  K      : ヘルプ (このページ)"),
        Line::from("  d/ESC  : TUI に戻る"),
        Line::from("  q      : 終了"),
        Line::from("  Ctrl+C : 強制終了"),
        Line::from(""),
        Line::from(Span::styled(
            "INSERT モード",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  ESC   : 確定 → NORMAL"),
        Line::from("  Enter : 確定 → 次小節 → INSERT 継続"),
        Line::from("  ;     : 分割して下の track に追加"),
        Line::from(""),
        Line::from(Span::styled(
            "  [ESC] でキャンセル",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    f.render_widget(
        Paragraph::new(help_lines).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" ヘルプ (Keybinds) ")
                .border_style(Style::default().fg(Color::Cyan)),
        ),
        popup,
    );
}
