use ratatui::{
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{
    memory, status::base_style, theme::MONOKAI_CYAN, ui::centered_text_block_rect,
};

const TITLE: &str = " Grid Sequencer ヘルプ(Keybinds)  Esc/q/?:close ";

/// 画面下部に常に出しておく1行のキーバインド要約。
pub(super) const KEYBIND_TEXT: &str =
    " r:randomize  R:randomize-notes  t:tracks  ?:help  Ctrl+G:screen  q:quit";

pub(super) fn draw_overlay(f: &mut Frame<'_>, track_count: usize) {
    let lines = help_lines(track_count);
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

fn help_lines(track_count: usize) -> Vec<Line<'static>> {
    // メモリ行は先頭に置く。端末が低いと centered_text_block_rect が下を切り落とすため。
    let mut lines = memory::overlay_lines();
    lines.extend(vec![
        Line::from(format!(
            "{track_count}行 x 16ステップ を常時ループ再生します。"
        )),
        Line::from("1ステップ = 250ms(16分音符)、16ステップ = 4秒で1周。"),
        Line::from(""),
        Line::from("  r        grid を丸ごとランダム設定(patch / note / 音長 / セル)"),
        Line::from("  R        patch を据え置き、note / 音長 / セルだけランダム設定"),
        Line::from("           (音色ロードが無いので再生が途切れない)"),
        Line::from("  t        track数を 1/2/4/8/16 で切替してアプリを再起動"),
        Line::from("  ?        このヘルプ"),
        Line::from("  Ctrl+G   画面切替メニュー"),
        Line::from("  q        終了"),
        Line::from(""),
        Line::from(format!(
            "行1〜{track_count}は realtime play server の CLAP instance 0〜{} に対応し、",
            track_count - 1
        )),
        Line::from(format!(
            "行ごとに別の音色で鳴ります。r は{track_count} instance ぶんの音色ロードを"
        )),
        Line::from("やり直すため、その間だけ再生が止まります。"),
        Line::from(""),
        Line::from("準備中の行は色が落ちます。暗いグレー = instance 未構築、"),
        Line::from("グレー = instance のみ構築済み、通常色 = 音色ロードまで完了。"),
    ]);
    lines
}
