//! どの画面からでも開ける MML 入力オーバーレイ。
//!
//! 打鍵ごとに「カーソル位置までの MML」を解釈し、音が変わったらその場で鳴らす。
//! MML 自体は揮発でよい前提のため保存先も履歴も持たない。音色だけは例外で、
//! 行頭の JSON（notepad 画面と同じ表現）として持ち、呼び出し側がセッションへ保存する。

pub mod patch_json;
pub(crate) mod patch_select;
pub mod prefix_notes;
mod sender;
mod state;
pub mod ui;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub use patch_select::is_patch_select_trigger;
pub use sender::MmlOverlaySender;
pub use state::{MmlOverlay, MmlOverlayAction};

/// このキーはどの画面からでも MML オーバーレイを開く。
pub fn is_mml_overlay_trigger(key: KeyEvent) -> bool {
    key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('p')
}
