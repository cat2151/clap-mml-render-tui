use crossterm::event::KeyEvent;
use ratatui_textarea::TextArea;

pub struct KeyboardMmlInput<'a> {
    active: bool,
    textarea: TextArea<'a>,
    last_confirmed: String,
    error: Option<String>,
}

impl Default for KeyboardMmlInput<'_> {
    fn default() -> Self {
        Self {
            active: false,
            textarea: cmrt_tui_core::text_input::new_single_line_textarea(""),
            last_confirmed: String::new(),
            error: None,
        }
    }
}

impl<'a> KeyboardMmlInput<'a> {
    pub fn is_active(&self) -> bool {
        self.active
    }

    pub fn textarea(&self) -> &TextArea<'a> {
        &self.textarea
    }

    pub fn value(&self) -> String {
        cmrt_tui_core::text_input::textarea_value(&self.textarea)
    }

    pub fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }

    pub fn open(&mut self) {
        self.textarea = cmrt_tui_core::text_input::new_single_line_textarea(&self.last_confirmed);
        self.error = None;
        self.active = true;
    }

    pub fn cancel(&mut self) {
        self.active = false;
        self.error = None;
    }

    pub fn input(&mut self, key: KeyEvent) {
        self.error = None;
        self.textarea.input(key);
    }

    pub fn confirm(&mut self) -> Option<Vec<Vec<u8>>> {
        let mml = self.value();
        match cmrt_chord::note_progression(&mml) {
            Ok(progression) => {
                self.last_confirmed = mml;
                self.error = None;
                self.active = false;
                Some(progression)
            }
            Err(error) => {
                self.error = Some(error);
                None
            }
        }
    }
}
