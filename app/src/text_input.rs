//! 1行テキスト入力（tui-textarea）のヘルパ。
//!
//! 実体は画面横断で共有するため `cmrt-tui-core` へ切り出した。
//! 従来の `crate::text_input::*` パスは再エクスポートで維持する。

pub(crate) use cmrt_tui_core::text_input::*;
