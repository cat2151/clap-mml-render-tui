use super::{KeyboardMidiSender, KeyboardMmlInput, KeyboardNoteGuide, KeyboardState};

/// Keyboard画面が所有する接続・入力・表示状態。
pub struct KeyboardScreen<'a> {
    pub midi_sender: Option<KeyboardMidiSender>,
    pub state: KeyboardState,
    pub mml_input: KeyboardMmlInput<'a>,
    pub note_guide: KeyboardNoteGuide,
}

impl<'a> KeyboardScreen<'a> {
    pub fn new(
        midi_sender: Option<KeyboardMidiSender>,
        state: KeyboardState,
        mml_input: KeyboardMmlInput<'a>,
        note_guide: KeyboardNoteGuide,
    ) -> Self {
        Self {
            midi_sender,
            state,
            mml_input,
            note_guide,
        }
    }
}
