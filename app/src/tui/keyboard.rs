use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::{Mode, PlayState, TuiApp};

mod catalog;
mod numeric_input;
mod sender;

use crate::history::{KeyboardSessionState, KeyboardTransport};
pub(super) use catalog::{KeyboardPatchCatalog, KeyboardPatchCatalogStatus};
pub(super) use numeric_input::{NumericInput, NumericInputTarget};
pub(super) use sender::{KeyboardConnectionPhase, KeyboardConnectionStatus, KeyboardMidiSender};

impl KeyboardConnectionPhase {
    fn accepts_notes(&self) -> bool {
        matches!(self, Self::Ready)
    }
}

pub(super) const KEYBOARD_NOTES: [KeyboardNote; 7] = [
    KeyboardNote::new('c', "C4", 60),
    KeyboardNote::new('d', "D4", 62),
    KeyboardNote::new('e', "E4", 64),
    KeyboardNote::new('f', "F4", 65),
    KeyboardNote::new('g', "G4", 67),
    KeyboardNote::new('a', "A4", 69),
    KeyboardNote::new('b', "B4", 71),
];

const NOTE_ON: u8 = 0x90;
const NOTE_OFF: u8 = 0x80;
const CONTROL_CHANGE: u8 = 0xB0;
const MODULATION_CC: u8 = 1;
const DEFAULT_VELOCITY: u8 = 100;
const ACCENT_VELOCITY: u8 = 127;
const DEFAULT_CC_NUMBER: u8 = MODULATION_CC;
const MODULATION_MAX: u8 = 127;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct KeyboardNote {
    pub(super) key: char,
    pub(super) name: &'static str,
    pub(super) midi_note: u8,
}

impl KeyboardNote {
    const fn new(key: char, name: &'static str, midi_note: u8) -> Self {
        Self {
            key,
            name,
            midi_note,
        }
    }
}

pub(crate) struct KeyboardState {
    held: Vec<KeyboardNote>,
    patch: Option<String>,
    transport: KeyboardTransport,
    buffer_multiplier: u8,
    velocity: u8,
    modulation_on: bool,
    cc_number: u8,
    numeric_input: Option<NumericInput>,
    pub(super) patch_catalog: KeyboardPatchCatalog,
}

impl Default for KeyboardState {
    fn default() -> Self {
        Self::new(None)
    }
}

impl KeyboardState {
    fn new(patch: Option<String>) -> Self {
        Self::from_session(KeyboardSessionState {
            patch,
            ..KeyboardSessionState::default()
        })
    }

    pub(super) fn from_session(session: KeyboardSessionState) -> Self {
        Self {
            held: Vec::new(),
            patch: session
                .patch
                .and_then(|patch| (!patch.trim().is_empty()).then_some(patch)),
            transport: session.transport,
            buffer_multiplier: session.buffer_multiplier,
            velocity: DEFAULT_VELOCITY,
            modulation_on: false,
            cc_number: DEFAULT_CC_NUMBER,
            numeric_input: None,
            patch_catalog: KeyboardPatchCatalog::default(),
        }
    }

    pub(super) fn session_state(&self) -> KeyboardSessionState {
        KeyboardSessionState {
            patch: self.patch.clone(),
            transport: self.transport,
            buffer_multiplier: self.buffer_multiplier,
        }
    }

    pub(super) fn held(&self) -> &[KeyboardNote] {
        &self.held
    }

    pub(super) fn patch(&self) -> Option<&str> {
        self.patch.as_deref()
    }

    pub(super) fn buffer_multiplier(&self) -> u8 {
        self.buffer_multiplier
    }

    pub(super) fn transport(&self) -> KeyboardTransport {
        self.transport
    }

    pub(super) fn velocity(&self) -> u8 {
        self.velocity
    }

    pub(super) fn modulation_on(&self) -> bool {
        self.modulation_on
    }

    pub(super) fn cc_number(&self) -> u8 {
        self.cc_number
    }

    pub(super) fn numeric_input(&self) -> Option<&NumericInput> {
        self.numeric_input.as_ref()
    }

    fn toggle_velocity(&mut self) -> u8 {
        self.velocity = if self.velocity == ACCENT_VELOCITY {
            DEFAULT_VELOCITY
        } else {
            ACCENT_VELOCITY
        };
        self.velocity
    }

    fn toggle_modulation(&mut self) -> [u8; 3] {
        self.modulation_on = !self.modulation_on;
        let value = if self.modulation_on {
            MODULATION_MAX
        } else {
            0
        };
        [CONTROL_CHANGE, MODULATION_CC, value]
    }

    pub(super) fn begin_numeric_input(&mut self, target: NumericInputTarget) {
        self.numeric_input = Some(NumericInput::new(target));
    }

    pub(super) fn numeric_input_push(&mut self, digit: char) {
        if let Some(input) = &mut self.numeric_input {
            input.push(digit);
        }
    }

    fn numeric_input_backspace(&mut self) {
        if let Some(input) = &mut self.numeric_input {
            input.backspace();
        }
    }

    fn cancel_numeric_input(&mut self) {
        self.numeric_input = None;
    }

    fn confirm_numeric_input(&mut self) -> Option<[u8; 3]> {
        let input = self.numeric_input.take()?;
        let value = input.confirmed_value()?;
        match input.target() {
            NumericInputTarget::CcNumber => {
                self.cc_number = value;
                None
            }
            NumericInputTarget::CcValue => Some([CONTROL_CHANGE, self.cc_number, value]),
        }
    }

    fn toggle_transport(&mut self) -> KeyboardTransport {
        self.transport = self.transport.toggled();
        self.transport
    }

    fn cycle_buffer_multiplier(&mut self) -> u8 {
        self.buffer_multiplier = match self.buffer_multiplier {
            1 => 2,
            2 => 4,
            4 => 8,
            _ => 1,
        };
        self.buffer_multiplier
    }

    pub(super) fn press(&mut self, note: KeyboardNote) -> Option<Vec<[u8; 3]>> {
        if self.held.iter().any(|held| held.key == note.key) {
            return None;
        }
        self.held.push(note);
        Some(vec![note_on(note, self.velocity)])
    }

    fn release(&mut self, note: KeyboardNote) -> Option<Vec<[u8; 3]>> {
        let index = self.held.iter().position(|held| held.key == note.key)?;
        self.held.remove(index);
        Some(vec![note_off(note)])
    }

    pub(super) fn take_reset_messages(&mut self) -> Vec<[u8; 3]> {
        let mut messages: Vec<[u8; 3]> = self.held.drain(..).map(note_off).collect();
        if std::mem::take(&mut self.modulation_on) {
            messages.push([CONTROL_CHANGE, MODULATION_CC, 0]);
        }
        messages
    }
}

fn note_for_key(code: KeyCode) -> Option<KeyboardNote> {
    let KeyCode::Char(key) = code else {
        return None;
    };
    KEYBOARD_NOTES.iter().find(|note| note.key == key).copied()
}

fn note_on(note: KeyboardNote, velocity: u8) -> [u8; 3] {
    [NOTE_ON, note.midi_note, velocity]
}

fn note_off(note: KeyboardNote) -> [u8; 3] {
    [NOTE_OFF, note.midi_note, 0]
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
            sender.prepare(
                self.keyboard_state.transport(),
                self.keyboard_state.buffer_multiplier(),
                self.keyboard_state.patch(),
            );
        }
    }

    pub(super) fn handle_keyboard_key_event(&mut self, key: KeyEvent) -> KeyboardAction {
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
                    self.keyboard_state.toggle_velocity();
                    return KeyboardAction::Continue;
                }
                KeyCode::Char('m') => {
                    if self.keyboard_connection_status().phase.accepts_notes() {
                        let message = self.keyboard_state.toggle_modulation();
                        if let Some(sender) = &self.keyboard_midi_sender {
                            sender.send(vec![message], self.keyboard_state.patch());
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
                    if let Some(sender) = &self.keyboard_midi_sender {
                        sender.switch(transport, note_offs, patch.as_deref());
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
}

#[cfg(test)]
mod tests;
