//! ヘルプ overlay。4.5 のキーバインドを全量出す。

use ratatui::{
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{status::base_style, theme::MONOKAI_CYAN, ui::centered_text_block_rect};

/// `cmrt_tui_core::buffer_test::help_overlay_bounds` が枠を探す目印でもあるので、
/// 「ヘルプ(Keybinds)」の並びは他画面と揃えたまま変えない。
const TITLE: &str = " Chord Chart ヘルプ(Keybinds)  Esc/?:close ";

/// 画面下段に常に出す 1 行の要約。
///
/// **載せるのは `q` と `?` の 2 つだけ**（`[user]` 指示）。キーを並べても覚えられず、
/// 溢れると末尾の `?:help`（全キーの見方への唯一の入口）が黙って切れる。
/// 全量はヘルプ overlay 側にある。
pub(super) const KEYBIND_TEXT: &str = " q:終了 ?:help";

pub(super) fn draw_overlay(f: &mut Frame<'_>) {
    let lines = help_lines();
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

/// ヘルプ overlay が教えるキーの全量。1 行は `キー` + 2 桁以上の空白 + `説明`。
///
/// **キーを足す / 消すときはここだけを直す。** 教えるキーの集合そのものを
/// `ui::tests::help::the_help_teaches_exactly_the_keys_that_survived_the_reduction` が
/// 固定しているので、廃止したキーが 1 文字（`a` `e` `x` `d`）で戻ってきても落ちる。
pub(super) const HELP_ROWS: [&str; 22] = [
    " Tab            pane 移動(Sections / Arrangement)",
    " j/k ↑↓         カーソル移動(行全体を試聴)",
    " h/l ←→         行内の chord 移動(その chord を試聴)",
    " PgUp/PgDn      カーソルを 10 行移動",
    " dd             カーソル行を削除",
    " Alt+↑/↓        カーソル行を上 / 下へ移動",
    " b              Key / BPM を入力",
    " Shift+P/Space  カーソル行の section を試聴(鳴っていたら停止)",
    " Ctrl+G         画面切替メニュー",
    " ? / Esc        このヘルプを開く / 閉じる",
    " q              アプリを終了",
    "",
    " ── Sections pane ──",
    " g              カタログから抽選して section 追加",
    " r              カーソル section の進行を抽選し直す",
    " i              進行(degrees)を編集",
    " n              名前を手入力",
    " dd             削除(arrangement 上の参照も一緒に消える)",
    "",
    " ── Arrangement pane ──",
    " 1..9           その番号の section をカーソルの次に挿入",
    " dd             カーソル行を削除",
];

/// pane ごとに見出しで区切る。同じ `dd` が pane で意味を変えるため、
/// キーだけを並べると読めない。
fn help_lines() -> Vec<Line<'static>> {
    HELP_ROWS.into_iter().map(Line::from).collect()
}
