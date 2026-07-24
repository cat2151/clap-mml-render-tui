//! OS クリップボードへの書き込み。
//!
//! 実体は画面横断で共有するため `cmrt-tui-core` へ切り出した。
//! 従来の `crate::clipboard::*` パスは再エクスポートで維持する。

pub(crate) use cmrt_tui_core::clipboard::*;
