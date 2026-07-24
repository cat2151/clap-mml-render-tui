pub(crate) mod clipboard;
pub mod config;
pub(crate) mod config_editor;
pub mod daw;
pub(crate) mod generate;
// セッション状態・patch history・voicing cache の永続化は notepad / DAW / keyboard で
// 共有するため `cmrt-history` crate へ切り出した。
// 従来の `crate::history::*` パスは再エクスポートで維持する。
pub use cmrt_history as history;
pub mod logging;
pub use cmrt_loop_browser_domain as loop_browser;
#[cfg(test)]
pub(crate) use cmrt_loop_domain::{loop_wav_analysis, loop_waveform};
pub(crate) mod mixer_overlay;
// オフラインレンダリング（in-process CLAP / render server）は notepad と DAW で
// 共有するため `cmrt-offline-render` crate へ切り出した。
// 従来の `crate::offline_render::*` パスは再エクスポートで維持する。
pub(crate) use cmrt_offline_render as offline_render;
pub(crate) mod patches;
pub(crate) mod random;
pub(crate) mod realtime_play;
pub(crate) mod screen_switch;
pub mod server;
pub(crate) mod sound_check_guide;
#[cfg(test)]
pub(crate) mod test_utils;
pub(crate) mod text_input;
pub mod tui;
pub(crate) mod ui_theme;
pub mod ui_utils;
pub mod updater;
pub mod voicing_cache_builder;
pub(crate) mod voicing_sources;
pub(crate) mod wav_io;
