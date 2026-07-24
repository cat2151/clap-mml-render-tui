//! 画面横断で共有する TUI 基盤プリミティブ。
//!
//! 特定画面（DAW / loop browser / keyboard 等）に依存しない、値ドメインと
//! 共通ウィジェットだけを集約する。
pub mod clipboard;
pub mod generate;
pub mod keyboard_session_state;
pub mod mixer;
pub mod mixer_overlay;
pub mod navigation;
pub mod patch_load;
pub mod patches;
pub mod play_state;
pub mod playback_session;
pub mod random;
pub mod screen_switch;
pub mod sound_check_guide;
pub mod status;
pub mod text_input;
pub mod theme;
pub mod ui;
pub mod wav_io;

pub use play_state::PlayState;
