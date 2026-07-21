use super::{KeyboardMidiSender, KeyboardMmlInput, KeyboardNoteGuide, KeyboardState};

/// Keyboard画面が所有する接続・入力・表示状態。
pub(in crate::tui) struct KeyboardScreen<'a> {
    pub(in crate::tui) midi_sender: Option<KeyboardMidiSender>,
    pub(in crate::tui) state: KeyboardState,
    pub(in crate::tui) mml_input: KeyboardMmlInput<'a>,
    pub(in crate::tui) note_guide: KeyboardNoteGuide,
    pub(in crate::tui) persist_on_exit: bool,
}

impl<'a> KeyboardScreen<'a> {
    pub(in crate::tui) fn new(
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
            persist_on_exit: false,
        }
    }
}
