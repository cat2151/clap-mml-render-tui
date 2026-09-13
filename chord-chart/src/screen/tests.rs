//! 画面のキー操作テストの共有ヘルパ。個々の観点は `tests/*.rs` へ。

use super::*;

use crossterm::event::KeyEvent;

mod cursor;
mod edit_degrees;
mod edit_keys;
mod enter;
mod help;

fn key(code: KeyCode) -> KeyEvent {
    KeyEvent::new(code, KeyModifiers::NONE)
}

/// section 3 つ・arrangement 2 行の曲。カーソルが両 pane で別々に動くことを見るため、
/// 行数をわざと変えてある。
fn three_section_screen() -> ChordChartScreen {
    let mut song = Song::empty();
    let a = song.push_section("A", "I-V-VIm-IV");
    let b = song.push_section("B", "IIm-V-I-VIm");
    song.push_section("Sabi", "IV-V-IIIm-VIm");
    song.arrangement = vec![a, b];
    ChordChartScreen::new(song)
}

/// section 25 個・arrangement 25 行の曲。`PgDn` / `PgUp` の 10 行を、端で丸められずに
/// 2 回ぶん見るために 21 行以上要る。
fn twenty_five_section_screen() -> ChordChartScreen {
    let mut song = Song::empty();
    let ids: Vec<_> = (0..25)
        .map(|index| song.push_section(format!("S{index}"), "I-V-VIm-IV"))
        .collect();
    song.arrangement = ids;
    ChordChartScreen::new(song)
}

/// section 1 つ（`A`）を 1 回だけ並べた曲。
fn one_section_song() -> Song {
    let mut song = Song::empty();
    let a = song.push_section("A", "I-V-VIm-IV");
    song.arrangement = vec![a];
    song
}

/// カタログを注入した画面。`progressions` が抽選の全候補。
fn with_catalog(mut screen: ChordChartScreen, progressions: &[&str]) -> ChordChartScreen {
    let progressions: Vec<String> = progressions
        .iter()
        .map(|text| (*text).to_string())
        .collect();
    screen.set_chord_progression_source(std::sync::Arc::new(move || progressions.clone()));
    screen
}
