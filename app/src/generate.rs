//! `g` キーで挿入する既定フレーズ。
//!
//! 実体は画面横断で共有するため `cmrt-tui-core` へ切り出した。
//! 従来の `crate::generate::*` パスは再エクスポートで維持する。

pub(crate) use cmrt_tui_core::generate::*;
