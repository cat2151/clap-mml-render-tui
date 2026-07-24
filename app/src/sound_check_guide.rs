//! 音出し確認ガイド（notepad / keyboard / DAW 共通）。
//!
//! 実体は画面横断で共有するため `cmrt-tui-core` へ切り出した。
//! 従来の `crate::sound_check_guide::*` パスは再エクスポートで維持する。

pub(crate) use cmrt_tui_core::sound_check_guide::*;
