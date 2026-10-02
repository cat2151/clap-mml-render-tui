//! keyboard 画面（PC キーボードを鍵盤として realtime play server へ MIDI を送る）。
//!
//! 画面の状態・入力・描画はこの crate に閉じており、共有ランタイム（app 側の `TuiApp`）
//! からは `KeyboardContext` で必要な情報を注入してもらう。app 側との接続は
//! app crate の `tui::keyboard_glue` にある。

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

mod catalog;
mod effect_pane;
pub mod guide;
mod logging;
mod mml_input;
mod navigation;
mod note_key;
mod numeric_input;
mod patch_filter_input;
mod periodic_timeline;
mod plugin_menu;
mod screen;
mod screen_runtime;
mod sender;
mod share_command;
mod share_notice;
// keyboard セッション状態の値型は、永続化層（`cmrt-history`）とも共有するため
// `cmrt-tui-core` が所有する。従来の `cmrt_keyboard::session_state::*` パスは
// 再エクスポートで維持する。
pub use cmrt_tui_core::keyboard_session_state as session_state;
mod state;
pub mod ui;

pub use catalog::{KeyboardPatchCatalog, KeyboardPatchCatalogStatus, PatchPaneFocus};
pub use effect_pane::KeyboardEffectPane;
pub use guide::KeyboardNoteGuide;
pub use logging::set_log_sink;
pub use mml_input::KeyboardMmlInput;
pub use navigation::NavigationCount;
pub use numeric_input::{NumericInput, NumericInputTarget};
pub use patch_filter_input::KeyboardPatchFilterInput;
pub use screen::KeyboardScreen;
pub use sender::{
    KeyboardConnectionPhase, KeyboardConnectionStatus, KeyboardMidiSender, KeyboardVoicingStatus,
};
pub use share_command::share_command;
pub use state::{KeyboardState, PeriodicTick};
pub use state::{ModulationMode, NotePlaybackMode, PitchBendMode, VelocityMode, KEYBOARD_NOTES};

use cmrt_realtime_play::PatchVoicing;
use cmrt_tui_core::patch_load::PatchLoadState;

impl KeyboardConnectionPhase {
    fn accepts_notes(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

/// 画面が返す、共有ランタイム側で処理すべき遷移要求。
pub enum KeyboardAction {
    Continue,
    ReturnToNotepad,
    LaunchDaw,
    Quit,
}

/// patch ごとの mono/poly 判定を、file cache などから解決する。
pub trait KeyboardVoicingLookup {
    fn cached_voicing(&self, patch: &str) -> Option<PatchVoicing>;
}

/// keyboard 画面が共有ランタイムから受け取る情報一式。
pub struct KeyboardContext<'a> {
    pub patch_dirs_configured: bool,
    /// patch 一覧のバックグラウンド読み込み状態。共有ランタイムの物をそのまま借りる。
    pub patch_load: &'a PatchLoadState,
    pub voicing: &'a dyn KeyboardVoicingLookup,
    /// 設定不足でカタログから外れたプラグインの案内。help 行の上へ出す。
    ///
    /// 一覧に**出てこない**ものの話なので、`patch_load` をいくら見ても分からない。
    /// 空なら 1 行も増えない。
    pub catalog_notes: &'a [String],
}

impl KeyboardContext<'_> {
    /// file cache に判定済みの mono/poly があれば返す。あれば probe を省略できる。
    pub fn cached_voicing(&self, patch: Option<&str>) -> Option<PatchVoicing> {
        self.voicing.cached_voicing(patch?)
    }
}

impl KeyboardScreen<'_> {
    /// 画面へ入るときの初期化。`patch` で選択音色を差し替える。
    pub fn start(&mut self, patch: Option<String>, ctx: &KeyboardContext<'_>) {
        self.mml_input.cancel();
        self.patch_filter.close();
        self.plugin_menu = None;
        self.effect.close_overlay();
        self.share_notice = None;
        self.note_guide.reset_for_screen();
        self.periodic_timeline.restart();
        self.state = self.state.restart_with_patch(patch);
        self.prepare_connection(ctx);
    }

    /// 直前の状態を保ったまま画面へ戻るときの初期化。
    pub fn resume(&mut self, ctx: &KeyboardContext<'_>) {
        self.mml_input.cancel();
        self.patch_filter.close();
        self.plugin_menu = None;
        self.effect.close_overlay();
        self.share_notice = None;
        self.note_guide.reset_for_screen();
        self.periodic_timeline.restart();
        self.prepare_connection(ctx);
    }

    /// 1 行入力欄（MML・絞り込み・effect の list 絞り込み）に打鍵している最中か。
    pub fn is_typing(&self) -> bool {
        self.mml_input.is_active() || self.patch_filter.is_active() || self.effect.is_typing()
    }

    /// 入力欄か overlay を開いていて、Ctrl+G の画面切替を開かない状態か。
    pub fn blocks_screen_switch(&self) -> bool {
        self.is_typing() || self.plugin_menu.is_some() || self.effect.is_adding()
    }

    pub fn prepare_connection(&self, ctx: &KeyboardContext<'_>) {
        if let Some(sender) = &self.midi_sender {
            let patch = self.state.patch();
            sender.prepare(
                self.state.buffer_multiplier(),
                patch,
                ctx.cached_voicing(patch),
                &cmrt_effect_chain_select::chain_json(self.effect.sounding()),
            );
        }
    }

    pub fn connection_status(&self) -> KeyboardConnectionStatus {
        self.midi_sender
            .as_ref()
            .map(KeyboardMidiSender::status)
            .unwrap_or_default()
    }

    /// worker スレッドが判定した voicing を画面状態へ取り込み、その接続状態を返す。
    /// 判定結果の file cache への書き戻しは共有ランタイム側の責務。
    pub fn sync_voicing_detection(&mut self) -> KeyboardConnectionStatus {
        let status = self.connection_status();
        self.state
            .set_detected_voicing(status.voicing.effective_decision());
        status
    }

    /// アプリ終了で保存する状態。
    pub fn session_state(&self) -> session_state::KeyboardSessionState {
        session_state::KeyboardSessionState {
            effect_chain: self.effect.chain().to_vec(),
            ..self
                .state
                .session_state(self.mml_input.last_confirmed().to_string())
        }
    }

    pub fn finish(&mut self) {
        let note_offs = self.state.take_leave_messages();
        self.stop_sending(note_offs);
    }

    pub fn handle_key(&mut self, key: KeyEvent, ctx: &KeyboardContext<'_>) -> KeyboardAction {
        // 共有の通知は次の Press で閉じる。そのキーは食わずに以下の通常処理へ流す。
        if key.kind == KeyEventKind::Press {
            self.share_notice = None;
        }
        if self.mml_input.is_active() {
            return self.handle_mml_input_key(key);
        }
        if self.patch_filter.is_active() {
            return self.handle_patch_filter_key(key, ctx);
        }
        if self.plugin_menu.is_some() {
            return self.handle_plugin_menu_key(key, ctx);
        }
        if self.effect.is_adding() {
            return self.handle_effect_add_key(key);
        }
        if key.kind == KeyEventKind::Repeat {
            return KeyboardAction::Continue;
        }
        // 数値入力モード中はPressを入力操作として消費する。Releaseだけは通常処理へ
        // 流し、押しっぱなしのノートが鳴りっぱなしになるのを防ぐ。
        if self.state.numeric_input().is_some() && key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Char(digit @ '0'..='9') => {
                    self.state.numeric_input_push(digit);
                }
                KeyCode::Backspace => {
                    self.state.numeric_input_backspace();
                }
                KeyCode::Esc => {
                    self.state.cancel_numeric_input();
                }
                KeyCode::Enter => {
                    let message = self.state.confirm_numeric_input();
                    if let Some(message) = message {
                        if self.connection_status().phase.accepts_notes() {
                            if let Some(sender) = &self.midi_sender {
                                sender.send(vec![message], self.state.patch());
                            }
                        }
                    }
                }
                _ => {}
            }
            return KeyboardAction::Continue;
        }
        if let Some(action) = self.handle_effect_pane_key(key) {
            return action;
        }
        if key.kind == KeyEventKind::Press {
            if key.modifiers == KeyModifiers::NONE {
                if let KeyCode::Char(digit @ '0'..='9') = key.code {
                    if self.state.navigation_count.push_digit(digit) {
                        return KeyboardAction::Continue;
                    }
                }
                match key.code {
                    KeyCode::Char('j') => {
                        let delta = self.state.navigation_count.take_delta(1);
                        self.move_focused_cursor(delta, ctx);
                        return KeyboardAction::Continue;
                    }
                    KeyCode::Char('k') => {
                        let delta = self.state.navigation_count.take_delta(-1);
                        self.move_focused_cursor(delta, ctx);
                        return KeyboardAction::Continue;
                    }
                    KeyCode::Char('l') => {
                        let delta = self.state.navigation_count.take_delta(1);
                        self.move_focus(delta, ctx);
                        return KeyboardAction::Continue;
                    }
                    KeyCode::Char('h') => {
                        let delta = self.state.navigation_count.take_delta(-1);
                        self.move_focus(delta, ctx);
                        return KeyboardAction::Continue;
                    }
                    _ => {}
                }
            } else if key.modifiers == KeyModifiers::CONTROL {
                match key.code {
                    KeyCode::Char('d') => {
                        let delta = self.state.navigation_count.take_delta(10);
                        self.move_focused_cursor(delta, ctx);
                        return KeyboardAction::Continue;
                    }
                    KeyCode::Char('u') => {
                        let delta = self.state.navigation_count.take_delta(-10);
                        self.move_focused_cursor(delta, ctx);
                        return KeyboardAction::Continue;
                    }
                    _ => {}
                }
            }
            self.state.navigation_count.clear();
        }
        if key.kind == KeyEventKind::Press
            && matches!(key.modifiers, KeyModifiers::NONE | KeyModifiers::SHIFT)
            && key.code == KeyCode::Char('/')
        {
            self.open_patch_filter(ctx);
            return KeyboardAction::Continue;
        }
        if plugin_menu::is_plugin_menu_key(key) {
            self.open_plugin_menu(ctx);
            return KeyboardAction::Continue;
        }
        if key.kind == KeyEventKind::Press
            && key.modifiers == KeyModifiers::SHIFT
            && matches!(key.code, KeyCode::Char('h' | 'H'))
        {
            let multiplier = self.state.cycle_buffer_multiplier();
            if let Some(sender) = &self.midi_sender {
                sender.set_buffer_multiplier(multiplier);
            }
            return KeyboardAction::Continue;
        }
        if key.kind == KeyEventKind::Press
            && key.modifiers == KeyModifiers::SHIFT
            && matches!(key.code, KeyCode::Char('z' | 'Z'))
        {
            if self.connection_status().phase.accepts_notes() {
                let message = self.state.toggle_cc_periodic(Instant::now());
                if let Some(sender) = &self.midi_sender {
                    sender.send(vec![message], self.state.patch());
                }
            }
            return KeyboardAction::Continue;
        }
        if key.kind == KeyEventKind::Press && key.modifiers == KeyModifiers::NONE {
            match key.code {
                KeyCode::Down => {
                    self.move_focused_cursor(1, ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::Up => {
                    self.move_focused_cursor(-1, ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::Right => {
                    self.move_focus(1, ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::Left => {
                    self.move_focus(-1, ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::PageDown => {
                    self.move_focused_cursor(10, ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::PageUp => {
                    self.move_focused_cursor(-10, ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::End => {
                    self.move_focused_to_end(ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::Home => {
                    self.move_focused_to_start(ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('v') => {
                    self.state.cycle_velocity(Instant::now());
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('m') => {
                    if self.connection_status().phase.accepts_notes() {
                        let message = self.state.cycle_modulation(Instant::now());
                        if let Some(sender) = &self.midi_sender {
                            sender.send(vec![message], self.state.patch());
                        }
                    }
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('p') => {
                    if self.connection_status().phase.accepts_notes() {
                        let message = self.state.cycle_pitch_bend(Instant::now());
                        if let Some(sender) = &self.midi_sender {
                            sender.send(vec![message], self.state.patch());
                        }
                    }
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('t') => {
                    if self.connection_status().phase.accepts_notes() {
                        let messages = self.state.cycle_note_playback(Instant::now());
                        self.send_after_cancel(messages);
                    }
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('y') => {
                    self.copy_share_command();
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('i') => {
                    self.mml_input.open();
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('x') => {
                    self.state.begin_numeric_input(NumericInputTarget::CcNumber);
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('z') => {
                    self.state.begin_numeric_input(NumericInputTarget::CcValue);
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('r')
                    if matches!(
                        self.connection_status().phase,
                        KeyboardConnectionPhase::Error(_)
                    ) =>
                {
                    self.state.take_reset_messages();
                    self.prepare_connection(ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('r') => {
                    self.select_random_patch(ctx);
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('n') => {
                    self.finish();
                    return KeyboardAction::ReturnToNotepad;
                }
                KeyCode::Char('w') => {
                    self.finish();
                    return KeyboardAction::LaunchDaw;
                }
                KeyCode::Char('q') => {
                    self.finish();
                    return KeyboardAction::Quit;
                }
                _ => {}
            }
        }
        self.handle_note_key(key)
    }

    fn handle_mml_input_key(&mut self, key: KeyEvent) -> KeyboardAction {
        if key.kind == KeyEventKind::Release {
            self.release_note_while_typing(key);
            return KeyboardAction::Continue;
        }

        match key.code {
            KeyCode::Esc => self.mml_input.cancel(),
            KeyCode::Enter => {
                if let Some(progression) = self.mml_input.confirm() {
                    let ready = self.connection_status().phase.accepts_notes();
                    let messages =
                        self.state
                            .replace_repeat_chords(progression, Instant::now(), ready);
                    if ready {
                        self.send_after_cancel(messages);
                    }
                }
            }
            _ => self.mml_input.input(key),
        }
        KeyboardAction::Continue
    }
}
