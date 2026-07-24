//! 主要画面の切替メニュー。
//!
//! 実体は画面横断で共有するため `cmrt-tui-core` へ切り出した。
//! 従来の `crate::screen_switch::*` パスは再エクスポートで維持する。

pub use cmrt_tui_core::screen_switch::*;
