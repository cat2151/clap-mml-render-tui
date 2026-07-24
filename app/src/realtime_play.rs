//! realtime play server の監督・送信（notepad 再生 / DAW / keyboard 共通）。
//!
//! 実体は画面横断で共有するため `cmrt-realtime-play` crate へ切り出した。
//! 従来の `crate::realtime_play::*` パスは再エクスポートで維持する。

pub(crate) use cmrt_realtime_play::*;
