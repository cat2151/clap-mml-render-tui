//! 音符キー（`c d e f g a b`）の note on / off。

use crossterm::event::{KeyEvent, KeyEventKind, KeyModifiers};

use super::{state::note_for_key, KeyboardAction, KeyboardScreen};

impl KeyboardScreen<'_> {
    /// 修飾なしの音符キーを鳴らす・止める。音符でないキーは何もしない。
    pub(crate) fn handle_note_key(&mut self, key: KeyEvent) -> KeyboardAction {
        if key.modifiers != KeyModifiers::NONE {
            return KeyboardAction::Continue;
        }
        let Some(note) = note_for_key(key.code) else {
            return KeyboardAction::Continue;
        };
        if !self.connection_status().phase.accepts_notes() {
            self.discard_random_chord_progression();
            self.state.take_reset_messages();
            return KeyboardAction::Continue;
        }
        let messages = match key.kind {
            KeyEventKind::Press => self.state.press(note),
            KeyEventKind::Release => self.state.release(note),
            KeyEventKind::Repeat => None,
        };
        if let (Some(messages), Some(sender)) = (messages, &self.midi_sender) {
            sender.send(messages, self.state.patch());
            if key.kind == KeyEventKind::Press {
                self.note_guide.complete();
            }
        }
        KeyboardAction::Continue
    }

    /// 入力欄が Press を食っている間も、Release は note off として通す。
    /// 押しっぱなしの音が鳴り続けるのを防ぐため。
    pub(crate) fn release_note_while_typing(&mut self, key: KeyEvent) {
        if key.modifiers != KeyModifiers::NONE {
            return;
        }
        let Some(note) = note_for_key(key.code) else {
            return;
        };
        if !self.connection_status().phase.accepts_notes() {
            self.discard_random_chord_progression();
            self.state.take_reset_messages();
            return;
        }
        if let (Some(messages), Some(sender)) = (self.state.release(note), &self.midi_sender) {
            sender.send(messages, self.state.patch());
        }
    }
}
