//! `?` の help overlay。左 pane にキーと一言、右 pane にキーを見ても分からない挙動を出す。
//! 背面の matrix と再生状態は変えない。

use cmrt_tui_core::{
    help_line::help_line,
    status::base_style,
    theme::{MONOKAI_CYAN, MONOKAI_GRAY},
    ui::{centered_rect_with_size, clear_overlay_area},
};
use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, Paragraph, Wrap},
    Frame,
};

/// `cmrt_tui_core::buffer_test::help_overlay_bounds` が枠を探す目印なので、
/// 「ヘルプ(Keybinds)」の並びは他画面と揃えたまま変えない。
const TITLE: &str = " Drum Sequencer ヘルプ(Keybinds)  Esc/?:close ";

pub(super) const KEY_ROWS: [&str; 17] = [
    " ── 編集 ──",
    " h/l ←→  step 移動",
    " k/j ↑↓  行移動",
    " Space   ON / OFF",
    " Enter   ON / OFF して右へ",
    " 数字    次のキーをくり返す",
    " - / +   長さ",
    " , / .   velocity",
    " [ / ]   pattern 切替",
    " ── 音色と再生 ──",
    " t       kit 選択",
    " Shift+P 再生 / 停止",
    " a       先読み時間",
    " ── 画面 ──",
    " Ctrl+G  画面切替",
    " q       終了",
    " ?       help を閉じる",
];

/// KEY_ROWS のキー欄の幅（行頭の空白を含む）。
pub(super) const KEY_COLUMN_WIDTH: usize = 9;

/// 80x24 の端末でも枠内へ収まるよう、幅 48 セル以内・行数は KEY_ROWS 程度に抑える。
pub(super) const DETAIL_ROWS: [&str; 20] = [
    " 行       上ほど高い note",
    " 左列     note number と構成音名(不明は ?)",
    " ON       one-shot は 1 step、他は 4 step",
    " Enter    右端では止まる",
    " 数字     例 2Enter 16k 12+ 3]。Space は捨てる",
    " -/+ ,/.  OFF なら同じ行の最寄りの x を変える",
    "          (等距離は左)",
    " 長さ     同じ note の次の x で切れる",
    "          one-shot は長さに関係なく鳴りきる",
    " velocity 8 ずつ 1〜127。ON 直後は 127",
    " pattern  0〜15。kit ごとに別。端で反対へ回る",
    " 保存     編集のたび kit・pattern ごとに .mid",
    " kit      低い note から試聴",
    "          替えるとその kit の pattern を読む",
    " 再生     2 秒周期の繰り返し",
    "          編集は先読みぶん先の step から反映",
    "          再生中に ON にしたセルはすぐ 1 回鳴る",
    " 先読み   0.05→0.1→0.25→0.5→1 秒",
    "          短いほど速く届き、音が抜けやすい",
    " 演奏位置 step 番号が反転、鳴っている x は黄色",
];

/// DETAIL_ROWS の用語欄の幅（行頭の空白を含む）。
pub(super) const DETAIL_COLUMN_WIDTH: usize = 10;

/// 枠線が食う幅・高さ。
const BORDER_SIZE: u16 = 2;

fn pane_width(rows: &[&str]) -> u16 {
    let width = rows.iter().map(|row| Line::from(*row).width()).max();
    u16::try_from(width.unwrap_or(0)).unwrap_or(u16::MAX)
}

/// 左 pane の幅。右 pane との間に 1 桁空ける。
fn key_pane_width() -> u16 {
    pane_width(&KEY_ROWS).saturating_add(1)
}

/// 2 pane を並べた枠。右 pane は左端の区切り線ぶん 1 桁広い。
fn overlay_rect(area: Rect) -> Rect {
    let content = key_pane_width()
        .saturating_add(1)
        .saturating_add(pane_width(&DETAIL_ROWS));
    let width = content
        .max(Line::from(TITLE).width() as u16)
        .saturating_add(BORDER_SIZE);
    let height = (KEY_ROWS.len().max(DETAIL_ROWS.len()) as u16).saturating_add(BORDER_SIZE);
    centered_rect_with_size(width, height, area)
}

pub(super) fn draw_overlay(frame: &mut Frame<'_>) {
    let area = overlay_rect(frame.area());
    let block = Block::default()
        .borders(Borders::ALL)
        .title(TITLE)
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_CYAN));
    let [keys, details] =
        Layout::horizontal([Constraint::Length(key_pane_width()), Constraint::Min(0)])
            .areas(block.inner(area));
    clear_overlay_area(frame, area);
    frame.render_widget(block, area);
    let lines = |rows: &[&'static str], key_width| {
        rows.iter()
            .map(|row| help_line(row, key_width))
            .collect::<Vec<_>>()
    };
    frame.render_widget(
        Paragraph::new(lines(&KEY_ROWS, KEY_COLUMN_WIDTH)).style(base_style()),
        keys,
    );
    // 端末が狭いときは右 pane が削られる。折り返して読めるようにしておく。
    frame.render_widget(
        Paragraph::new(lines(&DETAIL_ROWS, DETAIL_COLUMN_WIDTH))
            .style(base_style())
            .wrap(Wrap { trim: false })
            .block(
                Block::default()
                    .borders(Borders::LEFT)
                    .border_style(base_style().fg(MONOKAI_GRAY)),
            ),
        details,
    );
}
