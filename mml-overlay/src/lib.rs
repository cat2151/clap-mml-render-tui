//! どの画面からでも開ける MML 入力オーバーレイ。
//!
//! 入力欄は複数行で、1 行が 1 フレーズ。打鍵ごとにその瞬間の音が鳴り、カーソルが
//! 別の行へ移るとその行をまるごと鳴らす。書き並べたフレーズを上下キーで聴き比べる
//! ための画面なので、行は独立して解釈し、前の行のオクターブ等は引き継がない。
//!
//! MML 自体は揮発でよい前提のため保存先も履歴も持たない。音色だけは例外で、
//! 入力欄とは別に持ち（`Ctrl+T` で選ぶ）、呼び出し側がセッションへ保存する。
//! `Ctrl+O` で開くフレーズ履歴は notepad 画面と共有していて、読むだけ。

pub mod cursor_notes;
pub(crate) mod history_select;
pub mod line_play;
pub mod patch_json;
pub(crate) mod patch_select;
mod sender;
mod state;
pub mod ui;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub use history_select::is_history_select_trigger;
pub use patch_select::is_patch_select_trigger;
pub use sender::MmlOverlaySender;
pub use state::{MmlOverlay, MmlOverlayAction, MmlOverlayContext, PatchChange};

pub(crate) const NOTE_ON: u8 = 0x90;
pub(crate) const NOTE_OFF: u8 = 0x80;

/// このキーはどの画面からでも MML オーバーレイを開く。
pub fn is_mml_overlay_trigger(key: KeyEvent) -> bool {
    key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('p')
}
