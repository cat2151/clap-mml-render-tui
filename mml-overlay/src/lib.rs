//! どの画面からでも開ける MML 入力オーバーレイ。
//!
//! 入力欄は複数行で、1 行が 1 フレーズ。打鍵ごとにその瞬間の音が鳴り、カーソルが
//! 別の行へ移るとその行をまるごと鳴らす。書き並べたフレーズを上下キーで聴き比べる
//! ための画面なので、行は独立して解釈し、前の行のオクターブ等は引き継がない。
//!
//! MML 自体は揮発でよい前提のため保存先も履歴も持たない。音色だけは例外で、
//! 入力欄とは別に持ち（`Ctrl+T` で選ぶ）、呼び出し側がセッションへ保存する。
//! `Ctrl+O` で開くフレーズ履歴は notepad 画面と共有していて、読むだけ。

pub(crate) mod chord_transfer;
pub mod cursor_notes;
pub(crate) mod history_select;
pub mod line_play;
mod patch_catalog;
pub mod patch_json;
pub(crate) mod patch_select;
pub mod play_settings;
mod sender;
mod state;
pub mod ui;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub use history_select::is_history_select_trigger;
pub use patch_catalog::{host_patch_catalog, HostPatchCatalog, PatchCatalogEntry};
pub use patch_select::is_patch_select_trigger;
pub use play_settings::{is_play_settings_trigger, PlaySettings};
pub use sender::{MmlOverlaySender, MmlOverlaySenderStatus};
pub use state::{
    ChordPreviewContext, MmlOverlay, MmlOverlayAction, MmlOverlayContext, MmlOverlayInputMode,
    MmlOverlaySyntax, NoteRequest, PatchCatalogSnapshot, PatchChange,
};

pub(crate) const NOTE_ON: u8 = 0x90;
pub(crate) const NOTE_OFF: u8 = 0x80;

type LogSink = fn(&str);
static LOG_SINK: std::sync::OnceLock<LogSink> = std::sync::OnceLock::new();

/// app 起動時に、グローバルログ（`log/log.txt`）への書き込み関数を注入する。
/// 未注入の場合、この crate のログは黙って捨てられる。
///
/// 直接ファイルへ書かないのは、書き先が実ユーザーの `log/log.txt` 固定で、
/// 他 crate のテストからこの crate を通したときもそこへ追記してしまうため。
pub fn set_log_sink(log: LogSink) {
    let _ = LOG_SINK.set(log);
}

/// オーバーレイの調査ログ。1 行 1 事象で、キーと値を空白区切りで並べる。
pub(crate) fn log_line(message: String) {
    if let Some(sink) = LOG_SINK.get() {
        sink(&format!("mml-overlay: {message}"));
    }
}

#[cfg(test)]
mod tests;

/// このキーはどの画面からでも MML オーバーレイを開く。
pub fn is_mml_overlay_trigger(key: KeyEvent) -> bool {
    key.modifiers == KeyModifiers::CONTROL && key.code == KeyCode::Char('p')
}
