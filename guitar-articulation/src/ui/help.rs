//! ヘルプ overlay。この画面のキーと、画面の読み方を全量出す。

use ratatui::{
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph},
    Frame,
};

use cmrt_tui_core::{status::base_style, theme::MONOKAI_CYAN, ui::centered_text_block_rect};

/// `cmrt_tui_core::buffer_test::help_overlay_bounds` が枠を探す目印でもあるので、
/// 「ヘルプ(Keybinds)」の並びは他画面と揃えたまま変えない。
const TITLE: &str = " Guitar Articulation ヘルプ(Keybinds)  Esc/?:close ";

pub(crate) const HELP_ROWS: [&str; 24] = [
    " MML に打った音へ、列ごとにルールで奏法(KS)を足し、raw と聴き比べる画面。",
    "",
    " ── MML 欄(水色の枠のとき) ──",
    " 文字キー       MML を編集",
    " Enter          確定して Articulated を演奏",
    " Esc            編集を取り消して MML 欄を抜ける(matrix 操作へ)",
    "",
    " ── matrix(水色の枠のとき) ──",
    " h/l ←→         列移動(反転している列がカーソル)",
    " a              カーソル列の H/P を ON/OFF して Articulated を演奏",
    "                (前の列から上行→Hammer-On、下行→Pull-Off)",
    " e              イングヴェイ流エコノミーピッキング(eco 行)を行全体で ON/OFF して演奏",
    "                (同じ弦はオルタネイト、高い弦へはダウンで移る=スイープ、低い弦へは",
    "                 弦の最後をプリングにしてダウンで入る(D P D / D U P D)。D/U で表示。",
    "                 上行→下行の頂点だけ強く(ピンク)、他は弱める)",
    " s              自動H/P(auto 行)を行全体で ON/OFF して Articulated を演奏",
    "                (弦の最初はピッキング=p、以降は H/P)",
    "                 弦は、1 本で押さえる幅が 4 半音を超えたら次の弦へ移るとみなす",
    " b              raw(KS なし) を演奏",
    " space          Articulated(KS あり) を演奏",
    " x              effect chain を編集(ギターアンプは a → kind の Amp Simulator)",
    " i / q          MML 入力開始 / アプリを終了",
    "",
    " ? / Esc        このヘルプを開く / 閉じる   Ctrl+G  画面切替メニュー",
];

pub(super) fn draw_overlay(f: &mut Frame<'_>) {
    let lines: Vec<Line> = HELP_ROWS.into_iter().map(Line::from).collect();
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
