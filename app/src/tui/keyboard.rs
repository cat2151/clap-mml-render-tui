use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use super::{Mode, PlayState, TuiApp};

mod sender;

use crate::history::{KeyboardSessionState, KeyboardTransport};
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
const VELOCITY: u8 = 100;

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
        Some(vec![note_on(note)])
    }

    fn release(&mut self, note: KeyboardNote) -> Option<Vec<[u8; 3]>> {
        let index = self.held.iter().position(|held| held.key == note.key)?;
        self.held.remove(index);
        Some(vec![note_off(note)])
    }

    fn take_note_offs(&mut self) -> Vec<[u8; 3]> {
        self.held.drain(..).map(note_off).collect()
    }
}

fn note_for_key(code: KeyCode) -> Option<KeyboardNote> {
    let KeyCode::Char(key) = code else {
        return None;
    };
    KEYBOARD_NOTES.iter().find(|note| note.key == key).copied()
}

fn note_on(note: KeyboardNote) -> [u8; 3] {
    [NOTE_ON, note.midi_note, VELOCITY]
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
                KeyCode::Char('h') => {
                    let transport = self.keyboard_state.toggle_transport();
                    let patch = self.keyboard_state.patch().map(str::to_string);
                    let note_offs = self.keyboard_state.take_note_offs();
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
                    self.keyboard_state.take_note_offs();
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
            self.keyboard_state.take_note_offs();
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
        let note_offs = self.keyboard_state.take_note_offs();
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
mod tests {
    use super::*;

    #[test]
    fn note_mapping_is_c4_through_b4() {
        assert_eq!(
            KEYBOARD_NOTES
                .iter()
                .map(|note| (note.key, note.midi_note))
                .collect::<Vec<_>>(),
            vec![
                ('c', 60),
                ('d', 62),
                ('e', 64),
                ('f', 65),
                ('g', 67),
                ('a', 69),
                ('b', 71)
            ]
        );
    }

    #[test]
    fn notes_remain_active_until_each_key_is_released() {
        let mut state = KeyboardState::default();
        assert_eq!(state.press(KEYBOARD_NOTES[0]), Some(vec![[0x90, 60, 100]]));
        assert_eq!(state.press(KEYBOARD_NOTES[1]), Some(vec![[0x90, 62, 100]]));
        assert_eq!(state.held(), &KEYBOARD_NOTES[..2]);
        assert_eq!(state.release(KEYBOARD_NOTES[0]), Some(vec![[0x80, 60, 0]]));
        assert_eq!(state.held(), &KEYBOARD_NOTES[1..2]);
        assert_eq!(state.release(KEYBOARD_NOTES[1]), Some(vec![[0x80, 62, 0]]));
        assert!(state.held().is_empty());
    }

    #[test]
    fn duplicate_press_and_unknown_release_send_nothing() {
        let mut state = KeyboardState::default();
        assert!(state.press(KEYBOARD_NOTES[0]).is_some());
        assert_eq!(state.press(KEYBOARD_NOTES[0]), None);
        assert_eq!(state.release(KEYBOARD_NOTES[1]), None);
        assert_eq!(state.held(), &KEYBOARD_NOTES[..1]);
    }

    #[test]
    fn take_note_offs_stops_every_held_note_and_clears_state() {
        let mut state = KeyboardState::default();
        assert!(state.press(KEYBOARD_NOTES[0]).is_some());
        assert!(state.press(KEYBOARD_NOTES[2]).is_some());
        assert!(state.press(KEYBOARD_NOTES[4]).is_some());

        assert_eq!(
            state.take_note_offs(),
            vec![[0x80, 60, 0], [0x80, 64, 0], [0x80, 67, 0]]
        );
        assert!(state.held().is_empty());
    }

    #[test]
    fn keyboard_state_keeps_non_blank_patch() {
        let state = KeyboardState::new(Some("patches_factory/Keys/Piano.fxp".to_string()));

        assert_eq!(state.patch(), Some("patches_factory/Keys/Piano.fxp"));
        assert_eq!(KeyboardState::new(Some("  ".to_string())).patch(), None);
    }

    #[test]
    fn buffer_multiplier_defaults_to_x4_and_cycles_x8_x1_x2_x4() {
        let mut state = KeyboardState::default();
        assert_eq!(state.buffer_multiplier(), 4);
        assert_eq!(state.cycle_buffer_multiplier(), 8);
        assert_eq!(state.cycle_buffer_multiplier(), 1);
        assert_eq!(state.cycle_buffer_multiplier(), 2);
        assert_eq!(state.cycle_buffer_multiplier(), 4);
    }

    #[test]
    fn keyboard_state_defaults_to_shared_memory() {
        assert_eq!(
            KeyboardState::default().transport(),
            KeyboardTransport::SharedMemory
        );
    }
}
