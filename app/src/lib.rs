pub(crate) mod cached_source;
pub(crate) mod chord_progression_source;
pub mod config;
pub mod config_editor;
// DAW 画面本体は独立 crate へ切り出した。
// 従来の `crate::daw::*` パスは再エクスポートで維持する。
pub use cmrt_daw as daw;
// セッション状態・patch history・voicing cache の永続化は notepad / DAW / keyboard で
// 共有するため `cmrt-history` crate へ切り出した。
// 従来の `crate::history::*` パスは再エクスポートで維持する。
pub use cmrt_history as history;
pub mod logging;
pub use cmrt_loop_browser_domain as loop_browser;
#[cfg(test)]
pub(crate) use cmrt_loop_domain::{loop_wav_analysis, loop_waveform};
pub(crate) mod patches;
pub(crate) mod realtime_play;
pub(crate) mod screen_switch;
pub mod server;
pub(crate) mod sound_check_guide;
#[cfg(test)]
pub(crate) mod test_utils;
pub mod tui;
pub mod updater;
pub mod voicing_cache_builder;
pub(crate) mod voicing_sources;
