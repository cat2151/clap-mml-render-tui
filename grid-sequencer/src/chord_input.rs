//! 固定コード進行を受け取る、ratatui-textarea製の1行入力overlay。

use std::time::Instant;

use crossterm::event::{KeyCode, KeyEvent};
use ratatui_textarea::TextArea;

use crate::{GridSequencerContext, GridSequencerScreen};

pub(crate) struct ChordInputOverlay {
    textarea: TextArea<'static>,
    error: Option<String>,
}

impl ChordInputOverlay {
    fn new(text: &str) -> Self {
        Self {
            textarea: cmrt_tui_core::text_input::new_single_line_textarea(text),
            error: None,
        }
    }

    pub(crate) fn textarea(&self) -> &TextArea<'static> {
        &self.textarea
    }

    pub(crate) fn value(&self) -> String {
        cmrt_tui_core::text_input::textarea_value(&self.textarea)
    }

    pub(crate) fn error(&self) -> Option<&str> {
        self.error.as_deref()
    }
}

impl GridSequencerScreen {
    pub fn chord_input_open(&self) -> bool {
        self.chord_input.is_some()
    }

    pub(crate) fn chord_input_overlay(&self) -> Option<&ChordInputOverlay> {
        self.chord_input.as_ref()
    }

    pub(crate) fn open_chord_input(&mut self) {
        self.cancel_mouse_gesture();
        let initial = self
            .fixed_chord
            .as_ref()
            .map(|fixed| fixed.input().to_string())
            .or_else(|| {
                self.state
                    .chord()
                    .map(|chord| format!("key:{} {}", chord.key(), chord.degrees()))
            })
            .unwrap_or_default();
        self.chord_input = Some(ChordInputOverlay::new(&initial));
    }

    pub(crate) fn close_chord_input(&mut self) {
        self.chord_input = None;
    }

    pub(crate) fn handle_chord_input_key(
        &mut self,
        key: KeyEvent,
        now: Instant,
        ctx: &GridSequencerContext<'_>,
    ) {
        let Some(mut overlay) = self.chord_input.take() else {
            return;
        };
        match key.code {
            KeyCode::Esc => (),
            KeyCode::Enter => {
                let value = overlay.value();
                if let Err(error) = self.apply_fixed_chord_input(&value, now, ctx) {
                    overlay.error = Some(error);
                    self.chord_input = Some(overlay);
                }
            }
            _ => {
                if cmrt_tui_core::text_input::apply_key_event_to_textarea(
                    &mut overlay.textarea,
                    key,
                ) {
                    overlay.error = None;
                }
                self.chord_input = Some(overlay);
            }
        }
    }
}

#[cfg(test)]
mod tests;
