//! UI ユーティリティ（TUI / DAW 共通）
//!
//! 中央配置矩形ヘルパ（centered_rect / centered_rect_with_size / centered_text_block_rect）・
//! 音出し確認ガイドのオーバーレイ・カーソル移動先の先読み（predicted_navigation_indices 系）は、
//! 画面横断で共有するため `cmrt-tui-core` へ切り出した。
//! 従来の `crate::ui_utils::*` パスは再エクスポートで維持する。

pub(crate) use cmrt_tui_core::navigation::{
    predicted_navigation_indices, predicted_navigation_indices_with_direction_bias,
};
pub(crate) use cmrt_tui_core::ui::*;
