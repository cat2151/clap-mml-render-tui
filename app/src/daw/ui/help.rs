use ratatui::{
    layout::Rect,
    style::{Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::{MONOKAI_BG, MONOKAI_CYAN, MONOKAI_FG, MONOKAI_GRAY, MONOKAI_YELLOW};

const HELP_TITLE: &str = " ヘルプ (Keybinds) ";

pub(super) fn draw_help(f: &mut Frame, area: Rect, mode: super::super::DawMode) {
    let help_lines = match mode {
        super::super::DawMode::History => vec![
            Line::from(Span::styled(
                "HISTORY overlay",
                Style::default()
                    .fg(MONOKAI_YELLOW)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from("  ?        : ヘルプ (このページ)"),
            Line::from("  /        : MML 絞り込み開始"),
            Line::from("           : スペース区切りで AND 条件 (例: bass soft)"),
            Line::from("  Enter    : (検索中) 絞り込み入力を確定して操作に戻る"),
            Line::from("  n        : global history へ切り替え"),
            Line::from("  p        : current / selected patch history へ切り替え"),
            Line::from("  t        : patch select overlay へ切り替え"),
            Line::from("  h/l, ←/→ : History / Favorites 切り替え"),
            Line::from("  j/k, ↓/↑ : 項目移動して preview"),
            Line::from("  Space    : 現在項目を preview"),
            Line::from("  Enter    : (通常) 現在 track/meas に反映"),
            Line::from("  ESC      : 閉じる / 元の overlay に戻る"),
            Line::from(""),
            Line::from(Span::styled(
                "  [ESC] で戻る",
                Style::default().fg(MONOKAI_GRAY),
            )),
        ],
        super::super::DawMode::PatchSelect => vec![
            Line::from(Span::styled(
                "PATCH SELECT overlay",
                Style::default()
                    .fg(MONOKAI_YELLOW)
                    .add_modifier(Modifier::BOLD),
            )),
            Line::from("  ?        : ヘルプ (このページ)"),
            Line::from("  /        : patch name 絞り込み入力モード開始"),
            Line::from("           : (検索中) スペース区切りで AND 条件"),
            Line::from("  文字キー : (検索中) patch name を入力"),
            Line::from("  Enter    : (検索中) 絞り込みを確定"),
            Line::from("  ESC      : (検索中) 絞り込み入力を中断"),
            Line::from("  n        : global history へ切り替え"),
            Line::from("  p        : current / selected patch history へ切り替え"),
            Line::from("  t        : 現在選択 patch で開き直す"),
            Line::from("  h/l, ←/→ : (通常) Patches / Favorites 切り替えして preview"),
            Line::from("  j/k, ↓/↑ : (通常) 項目移動して preview"),
            Line::from("  Space    : (通常) 現在項目を preview"),
            Line::from("  Enter    : (通常) 現在 track の init meas patch を上書き"),
            Line::from("  ESC      : (通常) 閉じる"),
            Line::from(""),
            Line::from(Span::styled(
                "  [ESC] で戻る",
                Style::default().fg(MONOKAI_GRAY),
            )),
        ],
        _ => vec![
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
            Line::from("  u      : 直前の p を 1 回だけ取り消す"),
            Line::from("  g      : 現在 track/meas に generate を反映して preview"),
            Line::from("  e      : config.toml 編集 → 再起動"),
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
                "  [ESC] でキャンセル",
                Style::default().fg(MONOKAI_GRAY),
            )),
        ],
    };

    let popup = crate::ui_utils::centered_text_block_rect(area, HELP_TITLE, &help_lines);
    f.render_widget(Clear, popup);

    f.render_widget(
        Paragraph::new(help_lines)
            .style(Style::default().fg(MONOKAI_FG).bg(MONOKAI_BG))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(HELP_TITLE)
                    .border_style(Style::default().fg(MONOKAI_CYAN)),
            ),
        popup,
    );
}
