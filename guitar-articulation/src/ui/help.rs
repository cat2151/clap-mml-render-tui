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

pub(crate) const KEY_ROWS: [&str; 37] = [
    " ── MML 欄 ──",
    " 文字    MML を編集",
    " Enter   確定して演奏",
    " Esc     matrix 操作へ",
    " ── 列のルール ──",
    " h/l ←→  列移動して、その列の音を演奏(n に依らない)",
    " a       H/P",
    " m       パームミュート",
    " p       ピッキングハーモニクス",
    " /       スライド",
    " c       チョーキング",
    " v       ビブラート",
    " g       ピックスクレイプ",
    " t       奏法リスト(他の奏法も)",
    " ── 行のルール ──",
    " e       エコノミーピッキング",
    " s       自動H/P off/on1/on2",
    " d       汚し(humanize 行)",
    " Shift+A アクセント 上/下/上下",
    " r       汚し(リリース音)",
    " u       パラメータ(CC21 など)",
    " z       アルペジエーター",
    " ── 演奏・effect ──",
    " b       raw を演奏",
    " space   Articulated を演奏",
    " n       1 音モード(右 pane もその音だけ)",
    " Shift+R repeat 演奏",
    " x       effect chain を編集",
    " w       effect の dry/wet",
    " f       起動時の版 Lite→Full/Full",
    " Shift+H 履歴",
    " o       サンプル MID を開く",
    " Shift+O SMF を素材に読む",
    " Shift+M SMF を単音化",
    " i / q   MML 入力 / 終了",
    " ?       ヘルプを閉じる",
    " Ctrl+G  画面切替メニュー",
];

pub(crate) const DETAIL_ROWS: [&str; 68] = [
    " MML の音へ列ごとに奏法(KS)を足し、raw と聴き比べる。",
    " 反転した列がカーソル。ルールの切替で Articulated を演奏。",
    " ルールの段の文字は t の overlay の文字。灰色の - は効かない列(理由は枠の見出しに)",
    " ── 列のルール(カーソル列を ON/OFF) ──",
    " a  前の列から上行→Hammer-On、下行→Pull-Off",
    " m  Mute_Down/Up。効かない列(範囲外・和音など)は灰色",
    " p  PH(ピッキングハーモニクス)",
    " /  前の列から上行→Slide_Up、下行→Slide_Down。8 半音以上は 7 半音で滑る",
    " c  前の列から上行 1/2/3 半音→Bending_HT/WH/1HT",
    " v  CC20。a m p / c g は同じ列で 1 つだけ、v は重ねられる",
    " g  Pick_Scratch。音高は F#1〜F#2 へ畳み、和音は最低音だけ",
    " t  列ルールを matrix と同じ 5 段で並べる。j/k:段 h/l:動いた先を ON",
    "    (なし=段を OFF) 文字:ON/OFF space:試聴 /:絞り込み Enter/Esc:閉じる",
    "    harmonics=NH(ナチュラルハーモニクス)",
    "    brush=Brush_Down/Up  fret mute=Mute_Fret_D/U(D/U を保つ)",
    "    pseudo legato=Pseudo_Legato  portamento=Portament",
    "    (この 2 つは H/P の音も写す)。matrix では KS の段",
    "    slide in=Slide_In。幅は前の列からの音程(1〜7 半音)で CC27",
    "    最後に滑り下りて離す(スライドアウト)は 2 通り:",
    "    slide out=Slide_Out(E1 の KS、手動)。最後の列の音を、伸ばして",
    "    滑り下りる音に置き換える",
    "    auto slide out=CC24=88(自動)。列の奏法(H/P・slide 等)のまま、",
    "    離すときに滑り下りる。G#1〜E6、mute・PH 等には効かない。",
    "    position rel とは同じ列で 1 つだけ",
    "    trill half/whole/min3/maj3=Trill_HT/WT/min3/Maj3",
    "    unison bend/manual=Unison_Bend_Auto/Manual(C4〜C6)。manual は",
    "    pitch bend で持ち上げる。他の列と重なる列は効かない",
    "    chromatic run=Chromatic_Run。F#1〜F2 へ畳み、D#2〜F2 は効かない",
    "    slide fx down/up/wow=Slide_FX_D/U/Wow。音域へ畳む。down は",
    "    velocity で 3 層  fx hello/resonance/slide noise/hard stop=",
    "    note 0〜3 の効果音。g とこの 3 行は和音でも最低音だけ鳴らす",
    "    long/extra=CC23 power chord=CC32 position rel=CC24=72(重ね可)",
    " ── 行のルール(行全体を ON/OFF。e と s は排他) ──",
    " e  イングヴェイ流。同じ弦はオルタネイト、高い弦へは",
    "    ダウン(スイープ)、低い弦へは弦の最後をプリングにして",
    "    ダウンで入る(D P D / D U P D)。D/U/H/P で表示。",
    "    頭の音と上行→下行の頂点だけ強く(ピンク。Shift+A)、他の",
    "    ピッキングは弱める(H/P・mute・PH は弱めない)",
    " s  押すたびに off → on1 → on2。on1: 弦の最初はピッキング=p、",
    "    以降は H/P。1 本で押さえる幅が 4 半音を超えたら次の弦へ",
    "    移るとみなす。長い上行・下行は 3 音ごとと折り返しも p",
    "    on2: アクセント(Shift+A)の音だけ p、他は H/P。title に [auto2]",
    " d  e と同じ強弱に、音ごとの時刻・強さ・ノイズのばらつき。",
    "    列の記号は発音のずれ: < 早い / · ほぼジャスト / > 遅い",
    " Shift+A  e・d で強める音と s の on2 で弾く音を回す: 上=頂点 / 下=谷 / 上下=両方。",
    "    頭の音はどれでも強い。e・d・s の on2 の間 title に [accent:]",
    " r  列ごとに離した音の種類(Basic/Hard/Agressive/",
    "    Agressive2)と音量を乱数で。d とは独立。ON の列は ~",
    " u  CC21/22/28/29/46/48/52/53/112 を h/l で ±8。既定以外を頭で送る",
    " z  MML 欄はそのまま、音を低い順に並べて音型で鳴らす(変えるたびに演奏)。",
    "    開いている間は隙間なく repeat。各項目の説明は overlay の中の ?",
    " ── 演奏・effect ──",
    " n  ON の間は a/e/s/b/space がカーソル列の音だけ鳴らす",
    " Shift+R  ON の間、鳴らし終わるたびに同じ演奏を繰り返す(title に",
    "    [repeat])。0.5 秒未満の演奏は 1 周 0.5 秒まで待つ(z が ON なら待たず",
    "    アルペジオを隙間なく繋げる)。OFF で止める",
    " x  ギターアンプは a → kind の Amp Simulator。",
    "    Enter で確定すると wet に戻る",
    " w  chain を残して掛けずに鳴らす(title に [dry])。演奏する",
    "    音色の読み込み中は効かない",
    " Shift+H  j/k で選ぶたびに演奏。Enter で確定、Esc で戻す",
    " f  次に画面へ入ったとき最初に読む版(title の [起動:])。",
    "    Lite→Full は Lite ですぐ鳴らし裏で Full。保存される",
    " o  サンプル MID を選ぶ(j/k で試聴)。matrix に音と KS/CC/bend、",
    "    右 pane に全イベント。h/l で 1 音ずつ、space で全体を演奏。",
    "    Esc で MML へ戻る(MML・ルールはそのまま)",
    " O  SMF のパスを入れて Enter で読み、MML の代わりの素材に(note だけ・全 ch を ch1)",
    " M  SMF の高い音だけ残す(下の音は切り詰め)。i で MML を確定すると MML 素材へ戻る",
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
