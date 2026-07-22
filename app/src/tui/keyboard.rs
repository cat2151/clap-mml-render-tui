use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::{Mode, PlayState, TuiApp};

mod catalog;
pub(in crate::tui) mod guide;
mod mml_input;
mod navigation;
mod numeric_input;
mod screen;
mod screen_runtime;
mod sender;
mod state;

pub(super) use catalog::{KeyboardPatchCatalog, KeyboardPatchCatalogStatus};
pub(crate) use guide::KeyboardNoteGuide;
pub(crate) use mml_input::KeyboardMmlInput;
pub(in crate::tui) use navigation::NavigationCount;
pub(super) use numeric_input::{NumericInput, NumericInputTarget};
pub(in crate::tui) use screen::KeyboardScreen;
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
        self.keyboard.mml_input.cancel();
        self.keyboard.note_guide.reset_for_screen();
        self.voicing.layers = self.voicing.source_refresh.load_for_keyboard();
        let session = self.begin_playback_session();
        self.set_play_state_if_current(session, PlayState::Idle);
        self.keyboard.state = KeyboardState::new(patch);
        self.prepare_keyboard_connection();
        self.keyboard.persist_on_exit = false;
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
        if let Some(sender) = &self.keyboard.midi_sender {
            let patch = self.keyboard.state.patch();
            sender.prepare(
                self.keyboard.state.transport(),
                self.keyboard.state.buffer_multiplier(),
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
        self.voicing.layers.resolve(&self.voicing.cache, patch?)
    }

    pub(super) fn handle_keyboard_key_event(&mut self, key: KeyEvent) -> KeyboardAction {
        self.sync_keyboard_voicing_detection();
        if self.keyboard.mml_input.is_active() {
            return self.handle_keyboard_mml_input_key_event(key);
        }
        if key.kind == KeyEventKind::Repeat {
            return KeyboardAction::Continue;
        }
        // 数値入力モード中はPressを入力操作として消費する。Releaseだけは通常処理へ
        // 流し、押しっぱなしのノートが鳴りっぱなしになるのを防ぐ。
        if self.keyboard.state.numeric_input().is_some() && key.kind == KeyEventKind::Press {
            match key.code {
                KeyCode::Char(digit @ '0'..='9') => {
                    self.keyboard.state.numeric_input_push(digit);
                }
                KeyCode::Backspace => {
                    self.keyboard.state.numeric_input_backspace();
                }
                KeyCode::Esc => {
                    self.keyboard.state.cancel_numeric_input();
                }
                KeyCode::Enter => {
                    let message = self.keyboard.state.confirm_numeric_input();
                    if let Some(message) = message {
                        if self.keyboard_connection_status().phase.accepts_notes() {
                            if let Some(sender) = &self.keyboard.midi_sender {
                                sender.send(vec![message], self.keyboard.state.patch());
                            }
                        }
                    }
                }
                _ => {}
            }
            return KeyboardAction::Continue;
        }
        if key.kind == KeyEventKind::Press {
            if key.modifiers == KeyModifiers::NONE {
                if let KeyCode::Char(digit @ '0'..='9') = key.code {
                    if self.keyboard.state.navigation_count.push_digit(digit) {
                        return KeyboardAction::Continue;
                    }
                }
                match key.code {
                    KeyCode::Char('j') => {
                        let delta = self.keyboard.state.navigation_count.take_delta(1);
                        self.move_keyboard_patch_by(delta);
                        return KeyboardAction::Continue;
                    }
                    KeyCode::Char('k') => {
                        let delta = self.keyboard.state.navigation_count.take_delta(-1);
                        self.move_keyboard_patch_by(delta);
                        return KeyboardAction::Continue;
                    }
                    KeyCode::Char('l') => {
                        let delta = self.keyboard.state.navigation_count.take_delta(1);
                        self.move_keyboard_patch_category_by(delta);
                        return KeyboardAction::Continue;
                    }
                    KeyCode::Char('h') => {
                        let delta = self.keyboard.state.navigation_count.take_delta(-1);
                        self.move_keyboard_patch_category_by(delta);
                        return KeyboardAction::Continue;
                    }
                    _ => {}
                }
            } else if key.modifiers == KeyModifiers::CONTROL {
                match key.code {
                    KeyCode::Char('d') => {
                        let delta = self.keyboard.state.navigation_count.take_delta(10);
                        self.move_keyboard_patch_by(delta);
                        return KeyboardAction::Continue;
                    }
                    KeyCode::Char('u') => {
                        let delta = self.keyboard.state.navigation_count.take_delta(-10);
                        self.move_keyboard_patch_by(delta);
                        return KeyboardAction::Continue;
                    }
                    _ => {}
                }
            }
            self.keyboard.state.navigation_count.clear();
        }
        if key.kind == KeyEventKind::Press
            && key.modifiers == KeyModifiers::SHIFT
            && matches!(key.code, KeyCode::Char('h' | 'H'))
        {
            let multiplier = self.keyboard.state.cycle_buffer_multiplier();
            if let Some(sender) = &self.keyboard.midi_sender {
                sender.set_buffer_multiplier(multiplier);
            }
            return KeyboardAction::Continue;
        }
        if key.kind == KeyEventKind::Press
            && key.modifiers == KeyModifiers::SHIFT
            && matches!(key.code, KeyCode::Char('z' | 'Z'))
        {
            if self.keyboard_connection_status().phase.accepts_notes() {
                let message = self.keyboard.state.toggle_cc_periodic(Instant::now());
                if let Some(sender) = &self.keyboard.midi_sender {
                    sender.send(vec![message], self.keyboard.state.patch());
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
                    self.keyboard.state.cycle_velocity(Instant::now());
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('m') => {
                    if self.keyboard_connection_status().phase.accepts_notes() {
                        let message = self.keyboard.state.cycle_modulation(Instant::now());
                        if let Some(sender) = &self.keyboard.midi_sender {
                            sender.send(vec![message], self.keyboard.state.patch());
                        }
                    }
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('p') => {
                    if self.keyboard_connection_status().phase.accepts_notes() {
                        let message = self.keyboard.state.cycle_pitch_bend(Instant::now());
                        if let Some(sender) = &self.keyboard.midi_sender {
                            sender.send(vec![message], self.keyboard.state.patch());
                        }
                    }
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('t') => {
                    if self.keyboard_connection_status().phase.accepts_notes() {
                        let messages = self.keyboard.state.cycle_note_playback(Instant::now());
                        if !messages.is_empty() {
                            if let Some(sender) = &self.keyboard.midi_sender {
                                sender.send(messages, self.keyboard.state.patch());
                            }
                        }
                    }
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('i') => {
                    self.keyboard.mml_input.open();
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('x') => {
                    self.keyboard
                        .state
                        .begin_numeric_input(NumericInputTarget::CcNumber);
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('z') => {
                    self.keyboard
                        .state
                        .begin_numeric_input(NumericInputTarget::CcValue);
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('s') => {
                    let transport = self.keyboard.state.toggle_transport();
                    let patch = self.keyboard.state.patch().map(str::to_string);
                    let note_offs = self.keyboard.state.take_reset_messages();
                    let known_voicing = self.cached_voicing(patch.as_deref());
                    if let Some(sender) = &self.keyboard.midi_sender {
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
                    self.keyboard.state.take_reset_messages();
                    self.prepare_keyboard_connection();
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('r') => {
                    self.select_random_keyboard_patch();
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('n') => {
                    self.finish_keyboard();
                    self.keyboard.persist_on_exit = false;
                    self.mode = Mode::Normal;
                    self.reset_notepad_sound_check_guide();
                    return KeyboardAction::ReturnToNotepad;
                }
                KeyCode::Char('w') => {
                    self.finish_keyboard();
                    self.keyboard.persist_on_exit = false;
                    self.mode = Mode::Normal;
                    return KeyboardAction::LaunchDaw;
                }
                KeyCode::Char('q') => {
                    self.finish_keyboard();
                    self.keyboard.persist_on_exit = true;
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
            self.keyboard.state.take_reset_messages();
            return KeyboardAction::Continue;
        }
        let messages = match key.kind {
            KeyEventKind::Press => self.keyboard.state.press(note),
            KeyEventKind::Release => self.keyboard.state.release(note),
            KeyEventKind::Repeat => None,
        };
        if let (Some(messages), Some(sender)) = (messages, &self.keyboard.midi_sender) {
            sender.send(messages, self.keyboard.state.patch());
            if key.kind == KeyEventKind::Press {
                self.keyboard.note_guide.complete();
            }
        }
        KeyboardAction::Continue
    }

    fn handle_keyboard_mml_input_key_event(&mut self, key: KeyEvent) -> KeyboardAction {
        if key.kind == KeyEventKind::Release {
            if key.modifiers != KeyModifiers::NONE {
                return KeyboardAction::Continue;
            }
            let Some(note) = note_for_key(key.code) else {
                return KeyboardAction::Continue;
            };
            if !self.keyboard_connection_status().phase.accepts_notes() {
                self.keyboard.state.take_reset_messages();
                return KeyboardAction::Continue;
            }
            if let (Some(messages), Some(sender)) = (
                self.keyboard.state.release(note),
                &self.keyboard.midi_sender,
            ) {
                sender.send(messages, self.keyboard.state.patch());
            }
            return KeyboardAction::Continue;
        }

        match key.code {
            KeyCode::Esc => self.keyboard.mml_input.cancel(),
            KeyCode::Enter => {
                if let Some(progression) = self.keyboard.mml_input.confirm() {
                    let ready = self.keyboard_connection_status().phase.accepts_notes();
                    let messages = self.keyboard.state.replace_repeat_chords(
                        progression,
                        Instant::now(),
                        ready,
                    );
                    if ready && !messages.is_empty() {
                        if let Some(sender) = &self.keyboard.midi_sender {
                            sender.send(messages, self.keyboard.state.patch());
                        }
                    }
                }
            }
            _ => self.keyboard.mml_input.input(key),
        }
        KeyboardAction::Continue
    }

    pub(super) fn finish_keyboard(&mut self) {
        let note_offs = self.keyboard.state.take_reset_messages();
        if let Some(sender) = &self.keyboard.midi_sender {
            if !note_offs.is_empty() {
                sender.send(note_offs, self.keyboard.state.patch());
            }
            sender.stop();
        }
    }

    pub(super) fn keyboard_connection_status(&self) -> KeyboardConnectionStatus {
        self.keyboard
            .midi_sender
            .as_ref()
            .map(KeyboardMidiSender::status)
            .unwrap_or_default()
    }

    pub(in crate::tui) fn sync_keyboard_voicing_detection(&mut self) {
        let status = self.keyboard_connection_status();
        self.keyboard
            .state
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
        if !self.voicing.cache.insert(patch, report.decision) {
            return;
        }
        if let Err(error) = crate::history::save_voicing_cache(&self.voicing.cache) {
            log_voicing_cache_event(format!("event=save-failed patch={patch:?} error={error}"));
        }
    }
}
