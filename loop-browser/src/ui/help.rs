use ratatui::{
    style::Modifier,
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use crate::LoopBrowserPane;
use cmrt_tui_core::theme::{MONOKAI_CYAN, MONOKAI_GRAY, MONOKAI_YELLOW};

use super::base_style;

pub fn draw(frame: &mut Frame<'_>, pane: LoopBrowserPane) {
    let (title, lines) = match pane {
        LoopBrowserPane::Tree => tree_help(),
        LoopBrowserPane::Tracks => tracks_help(),
    };
    // メモリ行は先頭に置く。ヘルプが端末より長いと centered_text_block_rect が
    // 下を切り落とすため、末尾に置くと見えなくなる。
    let lines = [cmrt_tui_core::memory::overlay_lines(), lines].concat();

    let area = cmrt_tui_core::ui::centered_text_block_rect(frame.area(), title, &lines);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        area,
    );
}

fn tree_help() -> (&'static str, Vec<Line<'static>>) {
    (
        " ヘルプ (Keybinds): Loop Tree ",
        vec![
            section_title("Loop Tree pane"),
            Line::from("  Tab                 : Track List paneへ移動"),
            Line::from("  p                   : 演奏停止 / 再開"),
            Line::from("  r                   : ランダムWAVを選択"),
            Line::from("  Shift+O             : auto random（1周16秒以上なら1周、未満なら2周ごとにrandom化）"),
            Line::from("  Shift+C/D/E/F/G/A/B : 選択WAVをpadへ登録 / 解除"),
            Line::from("  c/d/e/f/g/a/b       : 登録済みpadを演奏"),
            Line::from("  数字 + h/j/k/l      : 指定回数移動 / 展開"),
            Line::from("  j/k, ↓/↑, PgDn/PgUp : 上下移動"),
            Line::from("  h/l, ←/→            : 折りたたみ / 展開"),
            Line::from("  Enter / Space       : 選択WAVを再生"),
            Line::from("  t                   : dirカテゴリ設定"),
            Line::from("  v / V               : お気に入り切替 / 限定表示"),
            Line::from("  Ctrl+G              : 画面切替"),
            Line::from("  q                   : 終了"),
            close_hint(),
        ],
    )
}

fn tracks_help() -> (&'static str, Vec<Line<'static>>) {
    (
        " ヘルプ (Keybinds): Tracks ",
        vec![
            section_title("Tracks pane"),
            Line::from("  Tab                 : Loop Tree paneへ移動"),
            Line::from("  p                   : 演奏停止 / 再開"),
            Line::from("  s                   : 選択trackのsolo切替"),
            Line::from("  Shift+M             : 内容のある2trackをrandom solo"),
            Line::from("  r                   : ランダムWAVで置換"),
            Line::from("  Shift+R             : 現在measureの全trackをrandom化"),
            Line::from("  Shift+O             : auto random（1周16秒以上なら1周、未満なら2周ごとにrandom化）"),
            Line::from("  m                   : mix level overlay"),
            Line::from("  c/d/e/f/g/a/b       : pad演奏 + 選択セル切替"),
            Line::from("  数字 + h/j/k/l      : 指定回数移動"),
            Line::from("  h/l, ←/→            : measure移動"),
            Line::from("  j/k, ↓/↑            : track移動"),
            Line::from("  Alt+↓/↑             : 選択trackを上下へ移動"),
            Line::from("                      : 右端 / 下端で自動追加"),
            Line::from("  Ctrl+G              : 画面切替"),
            Line::from("  q                   : 終了"),
            close_hint(),
        ],
    )
}

fn section_title(text: &'static str) -> Line<'static> {
    Line::from(Span::styled(
        text,
        base_style().fg(MONOKAI_YELLOW).add_modifier(Modifier::BOLD),
    ))
}

fn close_hint() -> Line<'static> {
    Line::from(Span::styled(
        "  [Esc / q / ?] でヘルプを閉じる",
        base_style().fg(MONOKAI_GRAY),
    ))
}
