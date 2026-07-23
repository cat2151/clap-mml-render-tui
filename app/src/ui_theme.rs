//! UI テーマ定義（TUI / DAW 共通）。
//!
//! 実体は画面横断で共有するため `cmrt-tui-core` へ切り出した。
//! 従来の `crate::ui_theme::*` パスは再エクスポートで維持する。

pub(crate) use cmrt_tui_core::theme::*;
