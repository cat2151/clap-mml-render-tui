//! 画面横断で共有する TUI 基盤プリミティブ。
//!
//! 特定画面（DAW / loop browser 等）に依存しない、値ドメインだけを集約する。
//! 現状は mixer の音量（dB）ドメインを提供する。
pub mod mixer;
