//! Keyboard の共通操作と Effect focus 固有操作を示す help overlay。

use ratatui::{
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{status::base_style, theme::MONOKAI_CYAN, ui::centered_text_block_rect};

const TITLE: &str = " Keyboard ヘルプ(Keybinds)  Esc/?:close ";

pub(super) fn draw_overlay(f: &mut Frame<'_>) {
    let lines: Vec<Line<'_>> = HELP_ROWS.into_iter().map(Line::from).collect();
    let area = centered_text_block_rect(f.area(), TITLE, &lines);
    let block = Block::default()
        .borders(Borders::ALL)
        .title(TITLE)
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_CYAN));
    let inner = block.inner(area);
    f.render_widget(Clear, area);
    f.render_widget(block, area);
    f.render_widget(Paragraph::new(lines).style(base_style()), inner);
}

const HELP_ROWS: [&str; 26] = [
    " ── 共通操作 ──",
    " c d e f g a b    音符を演奏(Effect focusでは c e f g)",
    " h/l ←/→         paneのfocusを移動",
    " j/k ↓/↑         focus中の項目を1行移動",
    " Ctrl+U/D PgUp/PgDn  10行移動",
    " 数字 + h/j/k/l・Ctrl+U/D  回数を指定して移動",
    " Home/End        focus中の先頭 / 末尾へ移動",
    " r               音色random選択 / 接続エラー時は再接続",
    " /               Patchesの絞り込み入力(Enter確定 / Esc取消)",
    " M               plugin solo/mute menu",
    " t               note playback: off / auto / repeat / arp",
    " v / m / p       velocity / modulation(CC1) / pitch bend mode切替",
    " x / z           CC番号 / CC値を入力(Enter確定 / Esc取消)",
    " Shift+Z         CC周期を切替",
    " Shift+H         buffer倍率を切替",
    " i               KeyboardのMML notes入力(Enter確定 / Esc取消)",
    " Shift+I         ランダムchord進行mode切替(有効化時に1つ生成)",
    " y               共有コマンドをコピー",
    " n / w           Notepad / DAWへ移動",
    " Ctrl+G / Ctrl+P  画面切替メニュー / 共通MML overlay(helpを閉じて利用)",
    " ? / Esc / q     help開閉 / helpを閉じる / 通常状態で終了",
    " ── Effect focusの操作(共通操作より優先) ──",
    " a / r           Effectを追加 / 差し替え(rはrandom・再接続より優先)",
    " dd              Effectを削除",
    " b               Effectのbypassを切替",
    " Alt+↑/↓         Effectの順序を上 / 下へ移動",
];
