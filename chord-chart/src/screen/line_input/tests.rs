//! 1 行入力のテストの共有ヘルパ。観点別のテストは `tests/*.rs` へ。
//!
//! degrees も prefix も**検証しない**ので、確定を止める理由が残っているのは
//! 「名前が空」の 1 つだけ。理由の出し方・消え方はそこで見る。

use super::*;

use crate::{ChordChartAction, Pane, Song};

mod prefix;
mod section;

fn key(code: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(code), KeyModifiers::NONE)
}

fn plain(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

fn ctrl(code: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(code), KeyModifiers::CONTROL)
}

/// section 2 つの曲。編集した section だけが変わることを見るため 2 つ持つ。
fn two_section_screen() -> ChordChartScreen {
    let mut song = Song::empty();
    let a = song.push_section("A", "I-V-VIm-IV");
    song.push_section("B", "IIm-V-I-VIm");
    song.arrangement = vec![a];
    ChordChartScreen::new(song)
}

/// 入力欄へ文字列を打ち込む。
fn type_text(screen: &mut ChordChartScreen, text: &str) {
    for ch in text.chars() {
        screen.handle_key_event(key(ch));
    }
}

/// 入力欄の中身を空にする（既定値が入っているので、打ち直しの前に消す）。
fn clear_input(screen: &mut ChordChartScreen) {
    for _ in 0..64 {
        screen.handle_key_event(plain(KeyCode::Backspace));
    }
}

fn input_value(screen: &ChordChartScreen) -> String {
    screen.line_input().expect("input is open").value()
}

fn degrees_of(screen: &ChordChartScreen, index: usize) -> &str {
    screen.song.sections[index].degrees.as_str()
}
