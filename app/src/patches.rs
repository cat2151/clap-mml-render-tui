//! patch（音色）の探索・カテゴリー分類・名前比較。
//!
//! 実体は画面横断で共有するため `cmrt-tui-core` へ切り出した。
//! 従来の `crate::patches::*` パスは再エクスポートで維持する。

pub(crate) use cmrt_tui_core::patches::*;
