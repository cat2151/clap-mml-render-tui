// NavigationCount は画面横断で共有するため `cmrt-tui-core` へ切り出した。
// 従来の `crate::NavigationCount` パスは再エクスポートで維持する。
pub use cmrt_tui_core::navigation::NavigationCount;
