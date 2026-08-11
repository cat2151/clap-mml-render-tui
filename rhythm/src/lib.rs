//! drum のリズム型と、その step 配置だけを組み立てる譜面モデル非依存のパターン生成。
//!
//! [`cmrt_arpeggiator`](https://docs.rs/cmrt-arpeggiator) と同じ設計思想で、出力は
//! 「step 何番で、何 step ぶん鳴らすか」という抽象表現に留めてある。音高も MIDI も
//! 持たないので、grid sequencer の譜面モデルには依存しない。
//!
//! 役割（[`DrumRole`]）ごとに型の list が分かれているのは、kick と hi-hat で意味のある
//! 刻みが違うため。wheel の list 送りはこの list の上を歩く。
//!
//! ```
//! use cmrt_rhythm::{generate_drum, DrumPattern, DrumRole, KickPattern};
//!
//! let hits = generate_drum(DrumPattern::Kick(KickPattern::Quarter), 16);
//! assert_eq!(hits.len(), 4);
//! assert_eq!(hits[0].step, 0);
//! // note は次の音が鳴るまで伸ばしっぱなしにする。
//! assert_eq!(hits[0].duration_steps, 4);
//!
//! assert_eq!(DrumPattern::default_for(DrumRole::Snare).label(), "Backbeat");
//! ```

mod pattern;
mod role;

pub use pattern::{
    generate_drum, DrumHit, DrumPattern, HatPattern, KickPattern, PercPattern, SnarePattern,
};
pub use role::DrumRole;
