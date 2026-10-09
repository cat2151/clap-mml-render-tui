//! Drum Sequencer の単一 matrix と、その pattern の SMF 形式。
//! 音色の選択・発音・保存先・画面の切替は host が担当する。

mod input;
mod pattern;
mod screen;
mod smf;
pub mod ui;

pub use pattern::{DrumHit, DrumPattern, DEFAULT_VELOCITY, PATTERN_COUNT};
pub use screen::{DrumSequencerScreen, KitResolution, DRUM_BPM, DRUM_STEPS};
pub use smf::{pattern_from_smf, pattern_to_smf};
