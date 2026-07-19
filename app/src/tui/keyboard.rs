use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::{Mode, PlayState, TuiApp};

mod catalog;
mod numeric_input;
mod sender;
mod state;

pub(super) use catalog::{KeyboardPatchCatalog, KeyboardPatchCatalogStatus};
pub(super) use numeric_input::{NumericInput, NumericInputTarget};
pub(super) use sender::{
    KeyboardConnectionPhase, KeyboardConnectionStatus, KeyboardMidiSender, KeyboardVoicingStatus,
};
pub(crate) use state::KeyboardState;
pub(super) use state::{
    ModulationMode, NotePlaybackMode, PitchBendMode, VelocityMode, KEYBOARD_NOTES,
};

use state::note_for_key;

fn log_voicing_cache_event(message: impl Into<String>) {
    #[cfg(not(test))]
    crate::logging::append_global_log_line(format!("voicing-cache: {}", message.into()));
    #[cfg(test)]
    let _ = message.into();
}

impl KeyboardConnectionPhase {
    fn accepts_notes(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

pub(super) enum KeyboardAction {
    Continue,
    ReturnToNotepad,
    LaunchDaw,
    Quit,
}

impl<'a> TuiApp<'a> {
    pub(super) fn start_keyboard_from_notepad(&mut self) {
        self.start_keyboard(self.current_line_patch_name());
    }

    pub(super) fn start_keyboard(&mut self, patch: Option<String>) {
        let session = self.begin_playback_session();
        self.set_play_state_if_current(session, PlayState::Idle);
        self.keyboard_state = KeyboardState::new(patch);
        self.prepare_keyboard_connection();
        self.persist_keyboard_on_exit = false;
        self.mode = Mode::Keyboard;
        self.is_daw_mode = false;
    }

    pub(super) fn prepare_restored_keyboard_connection(&self) {
        if self.mode == Mode::Keyboard
            && matches!(
                self.keyboard_connection_status().phase,
                KeyboardConnectionPhase::Idle
            )
        {
            self.prepare_keyboard_connection();
        }
    }

    fn prepare_keyboard_connection(&self) {
        if let Some(sender) = &self.keyboard_midi_sender {
            let patch = self.keyboard_state.patch();
            sender.prepare(
                self.keyboard_state.transport(),
                self.keyboard_state.buffer_multiplier(),
                patch,
                self.cached_voicing(patch),
            );
        }
    }

    /// file cache に判定済みの mono/poly があれば返す。あれば probe を省略できる。
    pub(in crate::tui) fn cached_voicing(
        &self,
        patch: Option<&str>,
    ) -> Option<crate::realtime_play::PatchVoicing> {
        self.voicing_cache.get(patch?)
    }

    pub(super) fn handle_keyboard_key_event(&mut self, key: KeyEvent) -> KeyboardAction {
        self.sync_keyboard_voicing_detection();
        if key.kind == KeyEventKind::Repeat {
            return KeyboardAction::Continue;
        }
        // 数値入力モード中はPressを入力操作として消費する。Releaseだけは通常処理へ
        // 流し、押しっぱなしのノートが鳴りっぱなしになるのを防ぐ。
        if self.keyboard_state.numeric_input().is_some() && key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Char(digit @ '0'..='9') => {
                    self.keyboard_state.numeric_input_push(digit);
                }
                KeyCode::Backspace => {
                    self.keyboard_state.numeric_input_backspace();
                }
                KeyCode::Esc => {
                    self.keyboard_state.cancel_numeric_input();
                }
                KeyCode::Enter => {
                    let message = self.keyboard_state.confirm_numeric_input();
                    if let Some(message) = message {
                        if self.keyboard_connection_status().phase.accepts_notes() {
                            if let Some(sender) = &self.keyboard_midi_sender {
                                sender.send(vec![message], self.keyboard_state.patch());
                            }
                        }
                    }
                }
                _ => {}
            }
            return KeyboardAction::Continue;
        }
        if key.kind == KeyEventKind::Press
            && key.modifiers == KeyModifiers::SHIFT
            && matches!(key.code, KeyCode::Char('h' | 'H'))
        {
            let multiplier = self.keyboard_state.cycle_buffer_multiplier();
            if let Some(sender) = &self.keyboard_midi_sender {
                sender.set_buffer_multiplier(multiplier);
            }
            return KeyboardAction::Continue;
        }
        if key.kind == KeyEventKind::Press
            && key.modifiers == KeyModifiers::SHIFT
            && matches!(key.code, KeyCode::Char('z' | 'Z'))
        {
            if self.keyboard_connection_status().phase.accepts_notes() {
                let message = self.keyboard_state.toggle_cc_periodic(Instant::now());
                if let Some(sender) = &self.keyboard_midi_sender {
                    sender.send(vec![message], self.keyboard_state.patch());
                }
            }
            return KeyboardAction::Continue;
        }
        if key.kind == KeyEventKind::Press && key.modifiers == KeyModifiers::NONE {
            match key.code {
                KeyCode::Down => {
                    self.move_keyboard_patch_by(1);
                    return KeyboardAction::Continue;
                }
                KeyCode::Up => {
                    self.move_keyboard_patch_by(-1);
                    return KeyboardAction::Continue;
                }
                KeyCode::PageDown => {
                    self.move_keyboard_patch_by(10);
                    return KeyboardAction::Continue;
                }
                KeyCode::PageUp => {
                    self.move_keyboard_patch_by(-10);
                    return KeyboardAction::Continue;
                }
                KeyCode::End => {
                    self.move_keyboard_patch_category_by(1);
                    return KeyboardAction::Continue;
                }
                KeyCode::Home => {
                    self.move_keyboard_patch_category_by(-1);
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('v') => {
                    self.keyboard_state.cycle_velocity(Instant::now());
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('m') => {
                    if self.keyboard_connection_status().phase.accepts_notes() {
                        let message = self.keyboard_state.cycle_modulation(Instant::now());
                        if let Some(sender) = &self.keyboard_midi_sender {
                            sender.send(vec![message], self.keyboard_state.patch());
                        }
                    }
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('p') => {
                    if self.keyboard_connection_status().phase.accepts_notes() {
                        let message = self.keyboard_state.cycle_pitch_bend(Instant::now());
                        if let Some(sender) = &self.keyboard_midi_sender {
                            sender.send(vec![message], self.keyboard_state.patch());
                        }
                    }
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('t') => {
                    if self.keyboard_connection_status().phase.accepts_notes() {
                        let messages = self.keyboard_state.cycle_note_playback(Instant::now());
                        if !messages.is_empty() {
                            if let Some(sender) = &self.keyboard_midi_sender {
                                sender.send(messages, self.keyboard_state.patch());
                            }
                        }
                    }
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('x') => {
                    self.keyboard_state
                        .begin_numeric_input(NumericInputTarget::CcNumber);
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('z') => {
                    self.keyboard_state
                        .begin_numeric_input(NumericInputTarget::CcValue);
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('h') => {
                    let transport = self.keyboard_state.toggle_transport();
                    let patch = self.keyboard_state.patch().map(str::to_string);
                    let note_offs = self.keyboard_state.take_reset_messages();
                    let known_voicing = self.cached_voicing(patch.as_deref());
                    if let Some(sender) = &self.keyboard_midi_sender {
                        sender.switch(transport, note_offs, patch.as_deref(), known_voicing);
                    }
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('r')
                    if matches!(
                        self.keyboard_connection_status().phase,
                        KeyboardConnectionPhase::Error(_)
                    ) =>
                {
                    self.keyboard_state.take_reset_messages();
                    self.prepare_keyboard_connection();
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('n') => {
                    self.finish_keyboard();
                    self.persist_keyboard_on_exit = false;
                    self.mode = Mode::Normal;
                    return KeyboardAction::ReturnToNotepad;
                }
                KeyCode::Char('w') => {
                    self.finish_keyboard();
                    self.persist_keyboard_on_exit = false;
                    self.mode = Mode::Normal;
                    return KeyboardAction::LaunchDaw;
                }
                KeyCode::Char('q') => {
                    self.finish_keyboard();
                    self.persist_keyboard_on_exit = true;
                    return KeyboardAction::Quit;
                }
                _ => {}
            }
        }
        if key.modifiers != KeyModifiers::NONE {
            return KeyboardAction::Continue;
        }
        let Some(note) = note_for_key(key.code) else {
            return KeyboardAction::Continue;
        };
        if !self.keyboard_connection_status().phase.accepts_notes() {
            self.keyboard_state.take_reset_messages();
            return KeyboardAction::Continue;
        }
        let messages = match key.kind {
            KeyEventKind::Press => self.keyboard_state.press(note),
            KeyEventKind::Release => self.keyboard_state.release(note),
            KeyEventKind::Repeat => None,
        };
        if let (Some(messages), Some(sender)) = (messages, &self.keyboard_midi_sender) {
            sender.send(messages, self.keyboard_state.patch());
        }
        KeyboardAction::Continue
    }

    pub(super) fn pump_keyboard_periodic(&mut self) {
        self.sync_keyboard_voicing_detection();
        if !self.keyboard_connection_status().phase.accepts_notes() {
            return;
        }
        // patch切替後の現在値再送(refresh) → 周期送信、の順で1回のsendにまとめる
        let now = Instant::now();
        let mut messages = self.keyboard_state.take_pending_refresh_messages(now);
        messages.extend(self.keyboard_state.poll_periodic(now));
        if !messages.is_empty() {
            if let Some(sender) = &self.keyboard_midi_sender {
                sender.send(messages, self.keyboard_state.patch());
            }
        }
    }

    pub(super) fn finish_keyboard(&mut self) {
        let note_offs = self.keyboard_state.take_reset_messages();
        if let Some(sender) = &self.keyboard_midi_sender {
            if !note_offs.is_empty() {
                sender.send(note_offs, self.keyboard_state.patch());
            }
            sender.stop();
        }
    }

    pub(super) fn keyboard_connection_status(&self) -> KeyboardConnectionStatus {
        self.keyboard_midi_sender
            .as_ref()
            .map(KeyboardMidiSender::status)
            .unwrap_or_default()
    }

    pub(in crate::tui) fn sync_keyboard_voicing_detection(&mut self) {
        let status = self.keyboard_connection_status();
        self.keyboard_state
            .set_detected_voicing(status.voicing.effective_decision());
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
        if !self.voicing_cache.insert(patch, report.decision) {
            return;
        }
        if let Err(error) = crate::history::save_voicing_cache(&self.voicing_cache) {
            log_voicing_cache_event(format!("event=save-failed patch={patch:?} error={error}"));
        }
    }
}
