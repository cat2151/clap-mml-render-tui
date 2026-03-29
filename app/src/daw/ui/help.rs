use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::{MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY, MONOKAI_YELLOW};

const HELP_POPUP_WIDTH_PERCENT: u16 = 82;
const HELP_POPUP_HEIGHT_PERCENT: u16 = 100;

pub(super) fn draw_help(f: &mut Frame, area: Rect) {
    let popup =
        crate::ui_utils::centered_rect(HELP_POPUP_WIDTH_PERCENT, HELP_POPUP_HEIGHT_PERCENT, area);
    f.render_widget(Clear, popup);

    let help_lines = vec![
        Line::from(Span::styled(
            "NORMAL モード",
            Style::default()
                .fg(MONOKAI_YELLOW)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  Shift+H: history overlay"),
        Line::from("  h / ← ・ l / → : 小節移動（非play時 preview）"),
        Line::from("  j / ↓  : track 移動（下, 非play時 preview）"),
        Line::from("  k / ↑  : track 移動（上, 非play時 preview）"),
        Line::from("  M      : 中央 track へ移動"),
        Line::from("  L      : 末尾 track へ移動"),
        Line::from("  i      : INSERT モード"),
        Line::from("  a      : off → start固定/end追従 → end固定 → off"),
        Line::from("  m      : mixer overlay"),
        Line::from("  dd     : 現在セルを yank して空にする（patch history 保存）"),
        Line::from("  p      : yank 内容で現在セルを上書き（上書き前は patch history 保存）"),
        Line::from("  Enter/Space : 非play時、現在trackの現在measを再生"),
        Line::from("  Shift+Enter : 非play時、現在measの全trackを再生"),
        Line::from("  Shift+P : 演奏 / 停止"),
        Line::from("  Shift+Space : 非play時、現在measから演奏開始して継続"),
        Line::from("  s      : solo toggle"),
        Line::from("  r      : random 音色設定"),
        Line::from("  K / ?  : ヘルプ (このページ)"),
        Line::from("  n      : notepad へ切替"),
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
            "HISTORY overlay",
            Style::default()
                .fg(MONOKAI_YELLOW)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from("  h/l, ←/→ : History / Favorites 切り替え"),
        Line::from("  j/k, ↓/↑ : 項目移動"),
        Line::from("  Enter    : 現在 track/meas に反映"),
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
