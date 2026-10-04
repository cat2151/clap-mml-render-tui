//! keyboard 画面（PC キーボードを鍵盤として realtime play server へ MIDI を送る）。
//!
//! 画面の状態・入力・描画はこの crate に閉じており、共有ランタイム（app 側の `TuiApp`）
//! からは `KeyboardContext` で必要な情報を注入してもらう。app 側との接続は
//! app crate の `tui::keyboard_glue` にある。

mod catalog;
mod effect_pane;
pub mod guide;
mod help;
mod input;
mod logging;
mod mml_input;
mod navigation;
mod note_key;
mod numeric_input;
mod patch_filter_input;
mod periodic_timeline;
mod plugin_menu;
mod random_chord;
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
pub use state::{ArpStep, KeyboardState, PeriodicTick, SoundingPosition};
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
        self.apply_random_chord_progression(std::time::Instant::now());
        self.discard_random_chord_progression();
        self.mml_input.cancel();
        self.patch_filter.close();
        self.plugin_menu = None;
        self.effect.close_overlay();
        self.help_open = false;
        self.share_notice = None;
        self.note_guide.reset_for_screen();
        self.periodic_timeline.restart();
        self.state = self.state.restart_with_patch(patch);
        self.prepare_connection(ctx);
    }

    /// 直前の状態を保ったまま画面へ戻るときの初期化。
    pub fn resume(&mut self, ctx: &KeyboardContext<'_>) {
        self.discard_random_chord_progression();
        self.mml_input.cancel();
        self.patch_filter.close();
        self.plugin_menu = None;
        self.effect.close_overlay();
        self.help_open = false;
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
        self.help_open || self.is_typing() || self.plugin_menu.is_some() || self.effect.is_adding()
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
        self.session_state_at(std::time::Instant::now())
    }

    pub(crate) fn session_state_at(
        &self,
        now: std::time::Instant,
    ) -> session_state::KeyboardSessionState {
        let mml = self
            .random_chord
            .pending_mml
            .as_ref()
            .filter(|(at, _)| *at <= now)
            .map_or_else(|| self.mml_input.last_confirmed(), |(_, mml)| mml);
        session_state::KeyboardSessionState {
            effect_chain: self.effect.chain().to_vec(),
            ..self.state.session_state_at(mml.to_string(), now)
        }
    }

    pub fn finish(&mut self) {
        self.apply_random_chord_progression(std::time::Instant::now());
        self.discard_random_chord_progression();
        let note_offs = self.state.take_leave_messages();
        self.stop_sending(note_offs);
    }
}
