//! 描画テストの共有ヘルパ。個々の観点は `tests/*.rs` へ。

use ratatui::{backend::TestBackend, buffer::Buffer, layout::Rect, Terminal};

use super::*;
use crate::{Pane, Section, SectionId, Song};

mod arrangement;
mod bass;
mod chord_highlight;
mod header;
mod help;
mod layout;
mod line_input;
mod readme;
mod sections;
mod text;

/// 80x24 の標準的な端末。列幅の閾値はここを基準に決めてある。
const TEST_WIDTH: u16 = 80;
const TEST_HEIGHT: u16 = 24;

fn render(screen: &ChordChartScreen) -> Buffer {
    render_sized(screen, TEST_WIDTH, TEST_HEIGHT)
}

fn render_sized(screen: &ChordChartScreen, width: u16, height: u16) -> Buffer {
    let mut terminal = Terminal::new(TestBackend::new(width, height)).unwrap();
    terminal.draw(|f| draw(screen, f)).unwrap();
    terminal.backend().buffer().clone()
}

fn buffer_to_string(buffer: &Buffer) -> String {
    (0..buffer.area.height)
        .map(|y| row_text(buffer, y))
        .collect::<Vec<_>>()
        .join("\n")
}

/// 全角は buffer 上でセル 2 つを占め、2 セル目が空白として読める（`不明` が
/// `不 明` になる）。**内容の照合は必ずこれを通す**。桁揃えを見るときだけ生の行を使う。
fn squeeze(text: &str) -> String {
    text.chars().filter(|ch| !ch.is_whitespace()).collect()
}

/// pane の中で「1 行 1 要素」として描かれている行だけを、空白と枠線を落として返す。
///
/// 判定は「左右の縦罫線に挟まれた中身が空でないこと」。列の中身（かつては `小節`）を
/// 目印にすると、列を 1 つ消しただけで全部のテストが道連れになる。
fn content_rows(pane: &str) -> Vec<String> {
    pane.lines()
        .map(squeeze)
        .filter(|line| line.starts_with('│'))
        .map(|line| line.trim_matches('│').to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

fn row_text(buffer: &Buffer, y: u16) -> String {
    (0..buffer.area.width)
        .map(|x| buffer.cell((x, y)).unwrap().symbol())
        .collect::<String>()
}

/// その pane の矩形に重なる行だけを、pane の幅で切り出す。
///
/// 左右 2 pane が同じ行に並ぶので、行全体を見ると「どちらの pane の文字か」が
/// 判別できない。テストは必ずこれを通す。
fn pane_text(buffer: &Buffer, pane: Pane) -> String {
    let layout = layout_for(Rect::new(0, 0, buffer.area.width, buffer.area.height));
    let area = match pane {
        Pane::Sections => layout.sections,
        Pane::Arrangement => layout.arrangement,
    };
    (area.y..area.y + area.height)
        .map(|y| {
            (area.x..area.x + area.width)
                .map(|x| buffer.cell((x, y)).unwrap().symbol())
                .collect::<String>()
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// section 2 つを 8 行に並べた曲。行数と重複参照を見るための土台。
fn song_of_eight_rows() -> Song {
    let mut song = Song::empty();
    let a = song.push_section("A", "I-V-VIm-IV");
    let b = song.push_section("B", "IIm-V-I-VIm");
    song.arrangement = vec![a, a, a, a, b, b, b, b];
    song
}

/// section 1 つ（`A`）を 1 回だけ並べた曲。
fn song_of_one_section() -> Song {
    let mut song = Song::empty();
    let a = song.push_section("A", "I-V-VIm-IV");
    song.arrangement = vec![a];
    song
}

fn screen_with(song: Song) -> ChordChartScreen {
    ChordChartScreen::new(song)
}

/// degrees が明らかに枠に収まらない section。
fn long_section(id: u32) -> Section {
    Section::new(
        SectionId::new(id),
        "Long",
        "I-V-VIm-IV-IIm-V-I-VIm-IV-V-IIIm-VIm-I-V-VIm-IV",
    )
}
