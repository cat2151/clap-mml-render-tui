//! どの画面からでも開ける MML 入力オーバーレイ。
//!
//! 打鍵ごとに「カーソル位置までの MML」を解釈し、音が変わったらその場で鳴らす。
//! 入力内容は揮発でよい前提のため、保存先も履歴も持たない。

pub mod prefix_notes;
mod sender;
mod state;
pub mod ui;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub use sender::MmlOverlaySender;
pub use state::{MmlOverlay, MmlOverlayAction};

/// このキーはどの画面からでも MML オーバーレイを開く。
pub fn is_mml_overlay_trigger(key: KeyEvent) -> bool {
    key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('p')
}
