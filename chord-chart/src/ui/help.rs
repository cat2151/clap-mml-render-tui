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
/// 80 桁端末（枠の内側 78 桁）に収まる長さで止めてある。溢れると末尾の `?:help`
/// が切れ、「全キーの見方」への入口が画面から消える。全量はヘルプ overlay 側。
///
/// **入りきらないので `b`（Key / BPM）はここに載せていない。** 曲ごとに 1 回決めれば
/// 済むうえ、現在値はヘッダに出ているので、キーだけが `?` 側にある。
pub(super) const KEYBIND_TEXT: &str =
    " hl:pane g:抽選 r:引直し i:進行 n:名前 dd:削除 Alt+↑↓:移動 q:終了 ?:help";

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
pub(super) const HELP_ROWS: [&str; 20] = [
    " h / l      pane 移動(h:Sections  l:Arrangement)",
    " j/k ↑↓     カーソル移動",
    " PgUp/PgDn  カーソルを 10 行移動",
    " dd         カーソル行を削除",
    " Alt+↑/↓    カーソル行を上 / 下へ移動",
    " b          Key / BPM を入力",
    " Ctrl+G     画面切替メニュー",
    " ? / Esc    このヘルプを開く / 閉じる",
    " q          アプリを終了",
    "",
    " ── Sections pane ──",
    " g          カタログから抽選して section 追加",
    " r          カーソル section の進行を抽選し直す",
    " i          進行(degrees)を手入力",
    " n          名前を手入力",
    " dd         削除(arrangement 上の参照も一緒に消える)",
    "",
    " ── Arrangement pane ──",
    " 1..9       その番号の section をカーソルの次に挿入",
    " dd         カーソル行を削除",
];

/// pane ごとに見出しで区切る。同じ `dd` が pane で意味を変えるため、
/// キーだけを並べると読めない。
fn help_lines() -> Vec<Line<'static>> {
    HELP_ROWS.into_iter().map(Line::from).collect()
}
