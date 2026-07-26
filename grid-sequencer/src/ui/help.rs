use ratatui::{
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{status::base_style, theme::MONOKAI_CYAN, ui::centered_text_block_rect};

const TITLE: &str = " Grid Sequencer ヘルプ(Keybinds)  Esc/q/?:close ";

/// 画面下部に常に出しておく1行のキーバインド要約。
pub(super) const KEYBIND_TEXT: &str = " r:randomize  ?:help  Ctrl+G:screen  q:quit";

pub(super) fn draw_overlay(f: &mut Frame<'_>) {
    let lines = help_lines();
    let area = centered_text_block_rect(f.area(), TITLE, &lines);
    f.render_widget(Clear, area);
    f.render_widget(
        Paragraph::new(lines).style(base_style()).block(
            Block::default()
                .borders(Borders::ALL)
                .title(TITLE)
                .style(base_style())
                .border_style(base_style().fg(MONOKAI_CYAN)),
        ),
        area,
    );
}

fn help_lines() -> Vec<Line<'static>> {
    [
        "16行 x 16ステップ を常時ループ再生します。",
        "1ステップ = 250ms(16分音符)、16ステップ = 4秒で1周。",
        "",
        "  r        grid を丸ごとランダム設定(patch / note / 音長 / セル)",
        "  ?        このヘルプ",
        "  Ctrl+G   画面切替メニュー",
        "  q        終了",
        "",
        "行の patch name は表示専用です。realtime play server は同時に",
        "1音色しか持てないため、実際の発音には行1の patch を使います。",
    ]
    .into_iter()
    .map(Line::from)
    .collect()
}
