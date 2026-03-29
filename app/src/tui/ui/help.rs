use ratatui::{
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use super::{base_style, MONOKAI_CYAN, MONOKAI_GRAY, MONOKAI_YELLOW};

pub(super) fn draw_help(f: &mut Frame) {
    let area = crate::ui_utils::centered_rect(60, 95, f.area());
    f.render_widget(Clear, area);

    let help_lines = vec![
        Line::from(Span::styled(
            "NORMAL モード",
            base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
        )),
        Line::from("  j / ↓       : 下へ移動して再生"),
        Line::from("  k / ↑       : 上へ移動して再生"),
        Line::from("  PageDown    : 1画面下へ移動して再生"),
        Line::from("  PageUp      : 1画面上へ移動して再生"),
        Line::from("  H           : 先頭行へ移動して再生"),
        Line::from("  M           : 中央行へ移動して再生"),
        Line::from("  L           : 末尾行へ移動して再生"),
        Line::from("  Enter/Space : 再生"),
        Line::from("  i           : INSERT モード"),
        Line::from("  o / O       : 下 / 上に挿入 → INSERT"),
        Line::from("  dd / Del : 削除（ヤンク）  p / P : 下貼付 / 上貼付"),
        Line::from("  r           : ランダム音色を挿入/置換して再生"),
        Line::from("  t           : 音色選択"),
        Line::from("  h           : notepad history"),
        Line::from("  f           : patch phrase 画面"),
        Line::from("  w           : DAW モード"),
        Line::from("  K / ?       : ヘルプ (このページ)"),
        Line::from("  q           : 終了"),
        Line::from(""),
        Line::from(Span::styled(
            "INSERT モード",
            base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
        )),
        Line::from("  ESC   : 確定 → NORMAL (再生)"),
        Line::from("  Enter : 確定 → 次行挿入 → INSERT 継続"),
        Line::from("  Ctrl+C: コピー"),
        Line::from("  Ctrl+X: カット"),
        Line::from("  Ctrl+V: ペースト"),
        Line::from(""),
        Line::from(Span::styled(
            "音色選択モード",
            base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
        )),
        Line::from("  文字入力 : フィルタ (Space=AND条件)"),
        Line::from("  Ctrl+J / Ctrl+N / ↓ : 下へ移動"),
        Line::from("  Ctrl+K / Ctrl+P / ↑ : 上へ移動"),
        Line::from("  PageUp / PageDown   : 1画面移動"),
        Line::from("  Enter   : 音色決定"),
        Line::from("  Ctrl+F  : 現在音色とMMLをFavorites追加"),
        Line::from("  ESC     : キャンセル"),
        Line::from(""),
        Line::from(Span::styled(
            "notepad history 画面",
            base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
        )),
        Line::from("  h / l ・ ← / → : ペイン切替"),
        Line::from("  j / k       : 上下移動して再生"),
        Line::from("  PageUp / PageDown : 1画面移動して再生"),
        Line::from("  Enter       : 現在行へ確定"),
        Line::from("  f           : History行をお気に入りに追加"),
        Line::from("  dd          : Favorites行を削除してHistory先頭へ移動"),
        Line::from("  ESC         : 閉じる"),
        Line::from(""),
        Line::from(Span::styled(
            "patch phrase 画面",
            base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
        )),
        Line::from("  j / k       : 上下移動して再生"),
        Line::from("  PageUp / PageDown : 1画面移動して再生"),
        Line::from("  h / l       : ペイン切替して再生"),
        Line::from("  Space       : 現在行を再生"),
        Line::from("  Enter       : 現在行の上に挿入"),
        Line::from("  i           : History行を編集"),
        Line::from("  f     : 現在行をお気に入りに追加"),
        Line::from("  ESC   : NORMAL に戻る"),
        Line::from(Span::styled(
            "  [ESC] でキャンセル",
            base_style().fg(MONOKAI_GRAY),
        )),
    ];

    f.render_widget(
        Paragraph::new(help_lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(" ヘルプ (Keybinds) ")
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        area,
    );
}
