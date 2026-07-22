//! loop browser のデータ/永続化層。
//!
//! UI（`crate::tui::loop_browser`）に依存しない、メタデータ・トラックグリッド・
//! ランダムデッキ・WAV ライブラリ走査・タイムストレッチ再生の各モジュールを束ねる。
pub mod library; // main.rs（scan-loops）から使われるため pub を維持
pub(crate) mod metadata;
pub(crate) mod persisted;
pub(crate) mod random;
pub(crate) mod time_stretch;
pub(crate) mod track_grid;
