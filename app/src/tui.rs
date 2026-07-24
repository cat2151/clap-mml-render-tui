//! 4画面（notepad / DAW / keyboard / loop browser）をホストする共有ランタイム。
//!
//! 各画面の実装本体は画面ごとの crate に閉じており、ここには
//! 「どの画面か」「画面をまたいで共有するもの」と、各 crate と接続する glue だけを置く。
//!
//! - notepad : `cmrt-notepad` crate（`notepad_glue`）
//! - keyboard: `cmrt-keyboard` crate（`keyboard_glue`）
//! - loop browser: `cmrt-loop-browser` crate（`loop_browser_glue`）
//! - DAW     : `crate::daw`（`runtime::screen` から起動）

// notepad 画面（状態・入力・描画・再生・レンダリングキュー）は `cmrt-notepad` crate へ
// 切り出した。従来の `crate::tui::notepad::*` パスは再エクスポートで維持する。
pub(crate) use cmrt_notepad as notepad;
mod notepad_glue;
// keyboard 画面（状態・入力・MIDI 送信・描画）は `cmrt-keyboard` crate へ切り出した。
// 従来の `crate::tui::keyboard::*` パスは再エクスポートで維持する。
pub(crate) use cmrt_keyboard as keyboard;
mod keyboard_glue;
// loop browser 画面（状態・入力・再生エンジン・描画）は `cmrt-loop-browser` crate へ切り出した。
// 従来の `crate::tui::loop_browser::*` パスは再エクスポートで維持する。
pub(crate) use cmrt_loop_browser as loop_browser;
mod loop_browser_glue;
mod runtime;
mod session;
mod ui;
mod voicing;

use ratatui::Frame;

use std::sync::{Arc, Mutex};

use cmrt_tui_core::playback_session::PlaybackSession;

use self::keyboard::KeyboardScreen;
use self::loop_browser::LoopBrowserScreen;
use self::notepad::{Mode, NormalAction, NotepadScreen};
use self::voicing::VoicingState;
use crate::config::Config;
use crate::screen_switch::{PrimaryScreen, ScreenSwitchMenu};
// PatchLoadState（notepad の音色選択と keyboard の patch catalog が共有）は
// `cmrt-tui-core` へ切り出した。
// 従来の `crate::tui::PatchLoadState` パスは再エクスポートで維持する。
pub(crate) use cmrt_tui_core::patch_load::PatchLoadState;
// PlayState は画面横断（notepad / loop browser）で共有するため `cmrt-tui-core` にある。
// 従来の `crate::tui::PlayState` パスは再エクスポートで維持する。
pub(crate) use cmrt_tui_core::PlayState;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TuiExitReason {
    Quit,
    RestartApp,
}

/// 4つの主要画面（notepad / DAW / keyboard / loop browser）をホストする共有ランタイム。
///
/// 各画面の状態は画面ごとの構造体に閉じており、ここが持つのは
/// 「どの画面か」「画面をまたいで共有するもの」だけ。
pub struct TuiApp<'a> {
    pub(super) active_screen: PrimaryScreen,
    pub(super) screen_switch_menu: ScreenSwitchMenu,
    cfg: Arc<Config>,
    entry_ptr: usize, // *const PluginEntry as usize。render_server backend では 0。
    pub(in crate::tui) notepad: NotepadScreen<'a>,
    pub(in crate::tui) keyboard: KeyboardScreen<'a>,
    pub(in crate::tui) loop_browser: LoopBrowserScreen,
    /// patch ごとの mono/poly 判定結果のキャッシュ。起動時に読み込み、
    /// 新しく probe した patch を検出したら書き戻す。keyboard 画面専用。
    pub(in crate::tui) voicing: VoicingState,
    /// バックグラウンドスレッドが収集したパッチリストの状態。
    /// notepad（音色選択）と keyboard（patch catalog）が同じ実体を読む。
    patch_load_state: Arc<Mutex<PatchLoadState>>,
    /// 再生セッションの世代管理。notepad・keyboard・loop browser で共有する。
    pub(in crate::tui) playback_session: PlaybackSession,
}

impl<'a> TuiApp<'a> {
    fn draw(&mut self, f: &mut Frame) {
        ui::draw(self, f);
    }
}

#[cfg(test)]
mod test_helpers;

#[cfg(test)]
mod tests;
