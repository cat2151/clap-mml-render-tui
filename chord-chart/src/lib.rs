//! コード進行の「構成」画面（Chord Chart）のドメインモデル。
//!
//! 1 曲を「名前付き section の定義」と「その並び（arrangement）」に分けて持つ。
//! 進行を `A` として 1 回定義し、arrangement には [`SectionId`] の参照だけを並べるので、
//! A を直せば曲中の全部の A が変わる。これが「1 曲は複数 set の組み合わせ」の実体。
//!
//! **この画面は degrees も prefix も一切解釈しない。** どちらも chord2mml-rs の
//! 書式をそのまま持つ文字列で、読めるかどうかを判断する責任は演奏側にある（別スコープ）。
//! だからこの crate はコード進行のパーサへ依存しない。
//!
//! **コード進行そのものをこの crate が持つこともしない。** 曲の中身は必ず
//! カタログ（`g` / `r`）か host 側の編集（`i`）から入る。
//!
//! ```
//! use cmrt_chord_chart::Song;
//!
//! // 初期値は空。既定の曲（＝進行のハードコード）は持たない。
//! let mut song = Song::empty();
//! assert_eq!(song.prefix, "Key=C BPM120");
//! assert!(song.sections.is_empty());
//!
//! // 中身はカタログから引いたものを入れる。
//! let picked_from_catalog = "(カタログから引いた進行)";
//! let id = song.push_section("A", picked_from_catalog);
//! song.arrangement = vec![id, id];
//! assert_eq!(song.arranged_sections().count(), 2);
//! ```

mod catalog;
mod persist;
mod screen;
mod song;
pub mod ui;

pub use catalog::ChordProgressionSource;
pub use persist::{load_song, save_song};
pub use screen::{ChordChartAction, ChordChartScreen, Pane, PreviewRequest, PreviewVoicingContext};
pub use song::{Section, SectionId, Song};
