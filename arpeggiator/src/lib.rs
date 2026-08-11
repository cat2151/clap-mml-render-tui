//! 声部の並びと各音の長さだけを組み立てる、譜面モデル非依存のフレーズ生成。
//!
//! アルペジオ（[`arp`]）とベースライン（[`bass`]）の2系統を持つ。どちらも出力は
//! 「step 何番で、何番目の声部を、何 step ぶん鳴らすか」という抽象表現に留めてある。
//! 声部を実際の note number へ解決するのは呼び出し側の責務なので、この crate は
//! grid sequencer の譜面モデルにも MIDI にも依存しない。
//!
//! ```
//! use cmrt_arpeggiator::{generate_arpeggio, generate_bass_line, ArpPattern, BassPattern};
//!
//! let notes = generate_arpeggio(ArpPattern::Up, 4, 16, &mut rand::rng());
//! assert_eq!(notes.len(), 16);
//! assert_eq!(notes[0].voice, 0);
//! assert_eq!(notes[1].voice, 1);
//!
//! // ベースラインは rng を引かない決定的な生成。
//! let bass = generate_bass_line(BassPattern::Whole, 16);
//! assert_eq!(bass.len(), 1);
//! assert_eq!(bass[0].duration_steps, 16);
//! ```

mod arp;
mod bass;

pub use arp::{clamp_durations, generate_arpeggio, ArpNote, ArpPattern, ARP_DURATION_CHOICES};
pub use bass::{generate_bass_line, BassNote, BassPattern, BASS_OCTAVE_VOICE, BASS_ROOT_VOICE};
