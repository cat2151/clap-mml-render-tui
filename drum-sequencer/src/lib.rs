//! Drum Sequencer の、実行中だけ保持する単一 matrix。
//! 音色の選択・発音・画面の切替は host が担当する。

mod input;
mod screen;
pub mod ui;

pub use screen::{DrumSequencerScreen, DRUM_STEPS};
