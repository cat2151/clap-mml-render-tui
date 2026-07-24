//! keyboard 画面と TuiApp（共有ランタイム）を接続する glue。
//!
//! 画面ロジック・MIDI 送信は `crate::tui::keyboard` 側にあり、ここは TuiApp の
//! フィールド（mode / active_screen / cfg / patch_load_state / voicing / playback）に
//! 触れる薄い接続層だけを残す。

use std::time::Instant;

use crossterm::event::KeyEvent;

use crate::realtime_play::PatchVoicing;
use crate::tui::keyboard::{
    KeyboardAction, KeyboardConnectionPhase, KeyboardConnectionStatus, KeyboardContext,
    KeyboardPatchLoad, KeyboardVoicingLookup, KeyboardVoicingStatus,
};
use crate::tui::voicing::VoicingState;
use crate::tui::{Mode, PatchLoadState, PlayState, TuiApp};

impl KeyboardVoicingLookup for VoicingState {
    fn cached_voicing(&self, patch: &str) -> Option<PatchVoicing> {
        self.layers.resolve(&self.cache, patch)
    }
}

fn keyboard_context<'ctx>(
    patch_dirs_configured: bool,
    patch_load: &'ctx PatchLoadState,
    voicing: &'ctx VoicingState,
) -> KeyboardContext<'ctx> {
    KeyboardContext {
        patch_dirs_configured,
        patch_load: match patch_load {
            PatchLoadState::Loading => KeyboardPatchLoad::Loading,
            PatchLoadState::Ready(pairs) => KeyboardPatchLoad::Ready(pairs),
            PatchLoadState::Err(error) => KeyboardPatchLoad::Err(error),
        },
        voicing,
    }
}

fn log_voicing_cache_event(message: impl Into<String>) {
    #[cfg(not(test))]
    crate::logging::append_global_log_line(format!("voicing-cache: {}", message.into()));
    #[cfg(test)]
    let _ = message.into();
}

impl<'a> TuiApp<'a> {
    fn patch_dirs_configured(&self) -> bool {
        crate::patches::has_configured_patch_dirs(&self.cfg)
    }

    pub(in crate::tui) fn start_keyboard_from_notepad(&mut self) {
        self.start_keyboard(self.current_line_patch_name());
    }

    pub(in crate::tui) fn start_keyboard(&mut self, patch: Option<String>) {
        self.voicing.layers = self.voicing.source_refresh.load_for_keyboard();
        let session = self.begin_playback_session();
        self.set_play_state_if_current(session, PlayState::Idle);

        let patch_dirs_configured = self.patch_dirs_configured();
        let patch_load = self.patch_load_state.lock().unwrap();
        let ctx = keyboard_context(patch_dirs_configured, &patch_load, &self.voicing);
        self.keyboard.start(patch, &ctx);
        drop(patch_load);

        self.mode = Mode::Keyboard;
        self.active_screen = crate::screen_switch::PrimaryScreen::Keyboard;
    }

    pub(in crate::tui) fn resume_keyboard(&mut self) {
        self.voicing.layers = self.voicing.source_refresh.load_for_keyboard();
        let session = self.begin_playback_session();
        self.set_play_state_if_current(session, PlayState::Idle);

        let patch_dirs_configured = self.patch_dirs_configured();
        let patch_load = self.patch_load_state.lock().unwrap();
        let ctx = keyboard_context(patch_dirs_configured, &patch_load, &self.voicing);
        self.keyboard.resume(&ctx);
        drop(patch_load);

        self.mode = Mode::Keyboard;
        self.active_screen = crate::screen_switch::PrimaryScreen::Keyboard;
    }

    pub(in crate::tui) fn prepare_restored_keyboard_connection(&self) {
        if self.mode != Mode::Keyboard
            || !matches!(
                self.keyboard.connection_status().phase,
                KeyboardConnectionPhase::Idle
            )
        {
            return;
        }
        let patch_dirs_configured = self.patch_dirs_configured();
        let patch_load = self.patch_load_state.lock().unwrap();
        let ctx = keyboard_context(patch_dirs_configured, &patch_load, &self.voicing);
        self.keyboard.prepare_connection(&ctx);
    }

    pub(in crate::tui) fn handle_keyboard_key_event(&mut self, key: KeyEvent) -> KeyboardAction {
        self.sync_keyboard_voicing_detection();

        let action = {
            let patch_dirs_configured = self.patch_dirs_configured();
            let patch_load = self.patch_load_state.lock().unwrap();
            let ctx = keyboard_context(patch_dirs_configured, &patch_load, &self.voicing);
            self.keyboard.handle_key(key, &ctx)
        };

        match &action {
            KeyboardAction::ReturnToNotepad => {
                self.mode = Mode::Normal;
                self.active_screen = crate::screen_switch::PrimaryScreen::Notepad;
                self.reset_notepad_sound_check_guide();
            }
            KeyboardAction::LaunchDaw => {
                self.mode = Mode::Normal;
                self.active_screen = crate::screen_switch::PrimaryScreen::Daw;
            }
            KeyboardAction::Continue | KeyboardAction::Quit => {}
        }
        action
    }

    pub(in crate::tui) fn pump_keyboard_periodic(&mut self) {
        self.sync_keyboard_voicing_detection();
        let first_overlay_today = self.keyboard.pump_periodic(
            Instant::now(),
            &crate::sound_check_guide::local_date_string(),
        );
        if first_overlay_today {
            self.save_keyboard_note_guide_overlay_date();
        }
    }

    pub(in crate::tui) fn finish_keyboard(&mut self) {
        self.keyboard.finish();
    }

    pub(in crate::tui) fn keyboard_connection_status(&self) -> KeyboardConnectionStatus {
        self.keyboard.connection_status()
    }

    pub(in crate::tui) fn sync_keyboard_patch_catalog(&mut self) {
        let patch_dirs_configured = self.patch_dirs_configured();
        let patch_load = self.patch_load_state.lock().unwrap();
        let ctx = keyboard_context(patch_dirs_configured, &patch_load, &self.voicing);
        self.keyboard.sync_patch_catalog(&ctx);
    }

    pub(in crate::tui) fn sync_keyboard_voicing_detection(&mut self) {
        let status = self.keyboard.sync_voicing_detection();
        self.store_probed_voicing(&status);
    }

    /// probe で新しく判定できた結果を file cache へ書き戻す。
    /// worker スレッドではなく UI スレッドで書くことで排他を不要にしている。
    fn store_probed_voicing(&mut self, status: &KeyboardConnectionStatus) {
        let KeyboardVoicingStatus::Detected(report) = &status.voicing else {
            return;
        };
        let Some(patch) = status.voicing_patch.as_deref() else {
            return;
        };
        if !self.voicing.cache.insert(patch, report.decision) {
            return;
        }
        if let Err(error) = crate::history::save_voicing_cache(&self.voicing.cache) {
            log_voicing_cache_event(format!("event=save-failed patch={patch:?} error={error}"));
        }
    }
}
