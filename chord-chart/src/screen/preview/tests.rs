//! preview 要求のテストの共有ヘルパ。観点別のテストは `tests/*.rs` へ。
//!
//! **音は出さない**（鳴らすのは app 側）。ここで見るのは「何を鳴らす要求が立つか」だけ。

use super::*;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::{ChordChartAction, Song};

mod bass;
mod cursor;
mod toggle;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// section 3 つ・arrangement 2 行（`A` → `Sabi`）。左右で行数と中身が違うので、
/// どちらの pane を見ているかが要求の中身で分かる。
fn screen() -> ChordChartScreen {
    let mut song = Song::empty();
    let a = song.push_section("A", "I-V-VIm-IV");
    song.push_section("B", "IIm-V-I-VIm");
    let sabi = song.push_section("Sabi", "IV-V-IIIm-VIm");
    song.arrangement = vec![a, sabi];
    ChordChartScreen::new(song)
}

/// 立っている要求を取り出す。立っていなければ不合格。
fn taken(screen: &mut ChordChartScreen) -> PreviewRequest {
    screen.take_preview().expect("preview 要求が立つこと")
}

/// 画面へ入った直後の 1 回を捨てて、キーの結果だけを見られる状態にする。
fn entered() -> ChordChartScreen {
    let mut screen = screen();
    screen.enter();
    screen.take_preview();
    screen
}
