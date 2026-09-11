//! chord カーソルの反転が、**画面のどの桁に出ているか**。
//!
//! 読むのは描画 buffer そのもの（span の割り方は `ui/degrees/tests.rs`）。
//! 反転しているセルを拾って文字列へ戻し、指している chord と突き合わせる。

use super::*;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Color, Modifier};

use crate::{ui::text::ELLIPSIS, SectionId};

/// 範囲の写しは glue が書き戻すもの。ここでは `-` で割って**テスト用に**組み立てる
/// （この crate は degrees を読まない。本物は `cmrt_chord::chord_source_ranges`）。
fn ranges_of(degrees: &str) -> Vec<std::ops::Range<usize>> {
    let mut ranges = Vec::new();
    let mut start = 0;
    for part in degrees.split('-') {
        ranges.push(start..start + part.len());
        start += part.len() + '-'.len_utf8();
    }
    ranges
}

/// 曲の全 section ぶんの写しを書き戻した画面（glue の代役）。
fn screen_with_ranges(song: Song) -> ChordChartScreen {
    let mut screen = ChordChartScreen::new(song);
    let ranges: Vec<(SectionId, _)> = screen
        .song
        .sections
        .iter()
        .map(|section| (section.id, ranges_of(&section.degrees)))
        .collect();
    screen.set_chord_ranges(ranges);
    screen
}

/// pane の中で反転しているセルを、行ごとに集めたもの（反転が無い行は落とす）。
fn reversed_rows(buffer: &Buffer, pane: Pane) -> Vec<String> {
    let layout = layout_for(Rect::new(0, 0, buffer.area.width, buffer.area.height));
    let area = match pane {
        Pane::Sections => layout.sections,
        Pane::Arrangement => layout.arrangement,
    };
    (area.y..area.y + area.height)
        .map(|y| {
            (area.x..area.x + area.width)
                .filter(|x| {
                    buffer
                        .cell((*x, y))
                        .unwrap()
                        .style()
                        .add_modifier
                        .contains(Modifier::REVERSED)
                })
                .map(|x| buffer.cell((x, y)).unwrap().symbol().to_string())
                .collect::<String>()
        })
        .filter(|row| !row.is_empty())
        .collect()
}

/// 反転しているのは**カーソルが指している chord 1 つだけ**。
#[test]
fn only_the_chord_the_cursor_points_at_is_reversed() {
    let mut screen = screen_with_ranges(song_of_eight_rows());
    screen.chord_cursor = 2;

    let rows = reversed_rows(&render(&screen), Pane::Sections);

    assert_eq!(rows, vec!["VIm".to_string()], "`I-V-VIm-IV` の 2 番目");
}

/// `l` を押すと反転が**右へ 1 chord ぶん**動く。押した結果として動くことを見る。
#[test]
fn pressing_l_moves_the_reversed_chord_one_to_the_right() {
    let mut screen = screen_with_ranges(song_of_eight_rows());

    assert_eq!(reversed_rows(&render(&screen), Pane::Sections), vec!["I"]);

    screen.handle_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
    assert_eq!(reversed_rows(&render(&screen), Pane::Sections), vec!["V"]);

    screen.handle_key_event(KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE));
    assert_eq!(reversed_rows(&render(&screen), Pane::Sections), vec!["VIm"]);

    screen.handle_key_event(KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE));
    assert_eq!(reversed_rows(&render(&screen), Pane::Sections), vec!["V"]);
}

/// **フォーカスしていない pane には反転が出ない。** chord カーソルはフォーカスしている
/// pane にしか無い（`Tab` で移ると先頭へ戻る）。
#[test]
fn the_pane_without_the_focus_shows_no_reversed_chord() {
    let mut screen = screen_with_ranges(song_of_eight_rows());
    screen.chord_cursor = 1;

    let buffer = render(&screen);

    assert_eq!(reversed_rows(&buffer, Pane::Sections), vec!["V"]);
    assert!(
        reversed_rows(&buffer, Pane::Arrangement).is_empty(),
        "{}",
        pane_text(&buffer, Pane::Arrangement)
    );
}

/// 右 pane では**参照先 section**の degrees を反転する（行そのものは進行を持たない）。
/// このとき左 pane には反転が出ない（フォーカスが無い側は素通し）。
#[test]
fn the_right_pane_reverses_a_chord_of_the_referenced_section() {
    let mut screen = screen_with_ranges(song_of_eight_rows());
    screen.handle_key_event(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
    // 5 行目 = `B`（`IIm-V-I-VIm`）の 2 番目 = `I`。左 pane の 0 行目は `A`
    // （`I-V-VIm-IV`）で、**同じ範囲を当てると別の綴り（`m`）になる**綴りを選んである。
    // 参照先で引けていなければ、ここが `VIm`（A の 2 番目）や `m` になって落ちる。
    screen.arrangement_cursor = 4;
    screen.chord_cursor = 2;

    let buffer = render(&screen);

    assert_eq!(reversed_rows(&buffer, Pane::Arrangement), vec!["I"]);
    assert!(
        reversed_rows(&buffer, Pane::Sections).is_empty(),
        "{}",
        pane_text(&buffer, Pane::Sections)
    );
}

/// 写しがまだ届いていない画面（`ChordChartScreen::new` のまま）は**反転しない**。
/// 読めない degrees（写しが 0 件）も同じ扱い。行は打ったとおりに出る。
#[test]
fn a_row_without_ranges_is_drawn_without_any_reversed_cell() {
    let mut song = Song::empty();
    let bad = song.push_section("Bad", "@@@");
    song.arrangement = vec![bad];

    // 写しが無い画面。
    let screen = ChordChartScreen::new(song.clone());
    let buffer = render(&screen);
    assert!(reversed_rows(&buffer, Pane::Sections).is_empty());
    assert!(
        pane_text(&buffer, Pane::Sections).contains("@@@"),
        "行は出る"
    );

    // 「切ったら 0 件だった」を書き戻した画面（読めない degrees）。
    let mut screen = ChordChartScreen::new(song);
    screen.set_chord_ranges([(bad, Vec::new())]);
    let buffer = render(&screen);
    assert!(reversed_rows(&buffer, Pane::Sections).is_empty());
    assert!(
        pane_text(&buffer, Pane::Sections).contains("@@@"),
        "行は出る"
    );
}

/// `…` で切れる長い行でも落ちない。**画面外の chord は反転が見えないだけ**。
#[test]
fn a_degrees_line_cut_by_the_ellipsis_never_panics() {
    let mut song = Song::empty();
    song.sections.push(long_section(1));
    song.arrangement.push(SectionId::new(1));
    let mut screen = screen_with_ranges(song);
    let last = screen.cursor_chord_count() - 1;

    // 末尾の chord（画面外）を指した状態で描く。
    screen.chord_cursor = last;
    let buffer = render(&screen);
    assert!(reversed_rows(&buffer, Pane::Sections).is_empty());
    assert!(
        pane_text(&buffer, Pane::Sections).contains(ELLIPSIS),
        "切れている"
    );

    // 幅を極端に狭めても同じ（pane の中身が 0 桁になる）。
    render_sized(&screen, 20, 10);
}

/// 端末が反転をどう描くか（fg と bg の入れ替え）を、そのまま計算に写す。
///
/// buffer には「反転しろ」という modifier しか入らない（色を入れ替えるのは端末）ので、
/// 色の読みやすさを機械で見るには、ここで同じ入れ替えを起こす必要がある。
fn rendered_colors(cell: &ratatui::buffer::Cell) -> (Color, Color) {
    let style = cell.style();
    let fg = style.fg.unwrap_or(Color::Reset);
    let bg = style.bg.unwrap_or(Color::Reset);
    if style.add_modifier.contains(Modifier::REVERSED) {
        (bg, fg)
    } else {
        (fg, bg)
    }
}

/// WCAG の相対輝度。色が `Rgb` でなければ比べようがないので `None`。
fn luminance(color: Color) -> Option<f64> {
    let Color::Rgb(r, g, b) = color else {
        return None;
    };
    let channel = |value: u8| {
        let value = f64::from(value) / 255.0;
        if value <= 0.03928 {
            value / 12.92
        } else {
            ((value + 0.055) / 1.055).powf(2.4)
        }
    };
    Some(0.2126 * channel(r) + 0.7152 * channel(g) + 0.0722 * channel(b))
}

/// WCAG のコントラスト比（1.0〜21.0）。
fn contrast(lhs: Color, rhs: Color) -> f64 {
    let lhs = luminance(lhs).expect("theme の色は Rgb");
    let rhs = luminance(rhs).expect("theme の色は Rgb");
    let (bright, dark) = if lhs > rhs { (lhs, rhs) } else { (rhs, lhs) };
    (bright + 0.05) / (dark + 0.05)
}

/// **行カーソルの強調と重なっても、反転した chord は読める。**
///
/// 「色の組み合わせが読めるか」は最後は人間の目だが、*数値として言えるところ*は
/// ここで固定する: 反転した桁は (1) 文字と背景のコントラストが本文と同等以上で、
/// (2) 同じ行の隣の桁と背景がはっきり違う。どちらかが崩れたら、実機を見るまでもなく
/// 落ちる（theme の色をいじったときの番人）。
#[test]
fn the_reversed_chord_stays_legible_on_the_highlighted_cursor_row() {
    let mut screen = screen_with_ranges(song_of_eight_rows());
    screen.chord_cursor = 2;
    let buffer = render(&screen);
    let layout = layout_for(Rect::new(0, 0, buffer.area.width, buffer.area.height));
    let area = layout.sections;

    // 反転がある行（＝カーソル行）だけを見る。同じ行の中で比べないと、
    // 「隣の桁」が強調されていない別の行のものになってしまう。
    let row = (area.y..area.y + area.height)
        .find(|y| {
            (area.x..area.x + area.width).any(|x| {
                buffer
                    .cell((x, *y))
                    .unwrap()
                    .style()
                    .add_modifier
                    .contains(Modifier::REVERSED)
            })
        })
        .expect("反転した桁がある行があること");
    let cells: Vec<_> = (area.x..area.x + area.width)
        .map(|x| buffer.cell((x, row)).unwrap())
        .collect();
    let (chord_fg, chord_bg) = cells
        .iter()
        .find(|cell| cell.style().add_modifier.contains(Modifier::REVERSED))
        .map(|cell| rendered_colors(cell))
        .expect("反転した桁があること");
    // 反転の**すぐ右**にある、同じ行の普通の桁（`I-V-VIm-IV` の `-IV`）。
    let (_, row_bg) = cells
        .iter()
        .skip_while(|cell| !cell.style().add_modifier.contains(Modifier::REVERSED))
        .find(|cell| !cell.style().add_modifier.contains(Modifier::REVERSED))
        .map(|cell| rendered_colors(cell))
        .expect("反転の右にも桁があること");
    assert!(
        contrast(chord_fg, chord_bg) >= 4.5,
        "反転した chord の文字が読めない: {chord_fg:?} on {chord_bg:?}"
    );
    assert!(
        contrast(chord_bg, row_bg) >= 3.0,
        "強調行の中で反転がどこか分からない: {chord_bg:?} vs {row_bg:?}"
    );
}
