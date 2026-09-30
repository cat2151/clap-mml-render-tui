//! ヘルプ overlay。左 pane にキーと一言、右 pane に画面の読み方と各キーの詳しい効き方を出す。

use ratatui::{
    layout::{Constraint, Layout, Rect},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use cmrt_tui_core::{
    status::base_style,
    theme::{MONOKAI_CYAN, MONOKAI_GRAY},
};

/// `cmrt_tui_core::buffer_test::help_overlay_bounds` が枠を探す目印でもあるので、
/// 「ヘルプ(Keybinds)」の並びは他画面と揃えたまま変えない。
const TITLE: &str = " Guitar Articulation ヘルプ(Keybinds)  Esc/?:close ";

pub(crate) const KEY_ROWS: [&str; 30] = [
    " ── MML 欄 ──",
    " 文字    MML を編集",
    " Enter   確定して演奏",
    " Esc     matrix 操作へ",
    " ── 列のルール ──",
    " h/l ←→  列移動",
    " a       H/P",
    " m       パームミュート",
    " p       ピッキングハーモニクス",
    " /       スライド",
    " c       チョーキング",
    " v       ビブラート",
    " g       ピックスクレイプ",
    " ── 行のルール ──",
    " e       エコノミーピッキング",
    " s       自動H/P",
    " d       汚し(humanize 行)",
    " r       汚し(リリース音)",
    " ── 演奏・effect ──",
    " b       raw を演奏",
    " space   Articulated を演奏",
    " n       1 音モード",
    " x       effect chain を編集",
    " w       effect の dry/wet",
    " f       起動時の版 Lite→Full/Full",
    " Shift+H 履歴",
    " o       サンプル MID を開く",
    " i / q   MML 入力 / 終了",
    " ?       ヘルプを閉じる",
    " Ctrl+G  画面切替メニュー",
];

pub(crate) const DETAIL_ROWS: [&str; 36] = [
    " MML の音へ列ごとに奏法(KS)を足し、raw と聴き比べる。",
    " 反転した列がカーソル。ルールの切替で Articulated を演奏。",
    " ── 列のルール(カーソル列を ON/OFF) ──",
    " a  前の列から上行→Hammer-On、下行→Pull-Off",
    " m  Mute_Down/Up。効かない列(範囲外・和音など)は灰色の -",
    " p  PH(ピッキングハーモニクス)",
    " /  前の列から上行 1〜7 半音→Slide_Up、下行→Slide_Down",
    " c  前の列から上行 1/2/3 半音→Bending_HT/WH/1HT",
    " v  CC20。a m p / c g は同じ列で 1 つだけ、v は重ねられる",
    " g  Pick_Scratch。音高は F#1〜F#2 へ畳み、和音は最低音だけ",
    "",
    " ── 行のルール(行全体を ON/OFF。e と s は排他) ──",
    " e  イングヴェイ流。同じ弦はオルタネイト、高い弦へは",
    "    ダウン(スイープ)、低い弦へは弦の最後をプリングにして",
    "    ダウンで入る(D P D / D U P D)。D/U/H/P で表示。",
    "    頭の音と上行→下行の頂点だけ強く(ピンク)、他の",
    "    ピッキングは弱める(H/P・mute・PH は弱めない)",
    " s  弦の最初はピッキング=p、以降は H/P。1 本で押さえる",
    "    幅が 4 半音を超えたら次の弦へ移るとみなす",
    " d  e と同じ強弱に、音ごとの時刻・強さ・ノイズのばらつき。",
    "    列の記号は発音のずれ: < 早い / · ほぼジャスト / > 遅い",
    " r  列ごとに離した音の種類(Basic/Hard/Agressive/",
    "    Agressive2)と音量を乱数で。d とは独立。ON の列は ~",
    "",
    " ── 演奏・effect ──",
    " n  ON の間は a/e/s/b/space がカーソル列の音だけ鳴らす",
    " x  ギターアンプは a → kind の Amp Simulator。",
    "    Enter で確定すると wet に戻る",
    " w  chain を残して掛けずに鳴らす(title に [dry])。演奏する",
    "    音色の読み込み中は効かない",
    " Shift+H  j/k で選ぶたびに演奏。Enter で確定、Esc で戻す",
    " f  次に画面へ入ったとき最初に読む版(title の [起動:])。",
    "    Lite→Full は Lite ですぐ鳴らし裏で Full。保存される",
    " o  付属のサンプル MID を選んで右 pane に全イベント。",
    "    space で全体、n ON なら h/l で 1 音ずつ演奏。",
    "    Esc で MML へ戻る(MML・ルールはそのまま)",
];

/// 左 pane の幅。枠の内側の区切り線を含まない。
fn key_pane_width() -> u16 {
    KEY_ROWS
        .iter()
        .map(|row| Line::from(*row).width())
        .max()
        .unwrap_or(0) as u16
        + 1
}

fn detail_pane_width() -> u16 {
    DETAIL_ROWS
        .iter()
        .map(|row| Line::from(*row).width())
        .max()
        .unwrap_or(0) as u16
}

/// 2 pane を並べた枠。画面に収まらないときは画面いっぱいまでに縮め、右 pane は折り返す。
fn overlay_rect(area: Rect) -> Rect {
    // 枠の左右 2 + 区切り線 1。
    let width = (key_pane_width() + 1 + detail_pane_width() + 2).min(area.width);
    let height = (KEY_ROWS.len().max(DETAIL_ROWS.len()) as u16 + 2).min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

/// (左 pane, 右 pane)。右 pane は左端の区切り線を含む。
pub(crate) fn pane_rects(area: Rect) -> (Rect, Rect) {
    let inner = help_block().inner(overlay_rect(area));
    let panes =
        Layout::horizontal([Constraint::Length(key_pane_width()), Constraint::Min(0)]).split(inner);
    (panes[0], panes[1])
}

fn help_block() -> Block<'static> {
    Block::default()
        .borders(Borders::ALL)
        .title(TITLE)
        .style(base_style())
        .border_style(base_style().fg(MONOKAI_CYAN))
}

pub(super) fn draw_overlay(f: &mut Frame<'_>) {
    let area = overlay_rect(f.area());
    let (keys, details) = pane_rects(f.area());
    f.render_widget(Clear, area);
    f.render_widget(help_block(), area);
    let lines = |rows: &[&'static str]| rows.iter().copied().map(Line::from).collect::<Vec<_>>();
    f.render_widget(Paragraph::new(lines(&KEY_ROWS)).style(base_style()), keys);
    f.render_widget(
        Paragraph::new(lines(&DETAIL_ROWS))
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
