use std::time::Instant;

use cmrt_tui_core::bpm::{BpmInputAction, BpmMode};
use crossterm::event::KeyEvent;

use crate::GridSequencerScreen;

impl GridSequencerScreen {
    pub(crate) fn handle_bpm_input_key(&mut self, key: KeyEvent, now: Instant) {
        let Some(input) = self.bpm_input.as_mut() else {
            return;
        };
        match input.handle_key(key) {
            BpmInputAction::Continue => {}
            BpmInputAction::Cancel => self.bpm_input = None,
            BpmInputAction::Apply(mode) => self.apply_bpm_mode(mode, now),
        }
    }

    fn apply_bpm_mode(&mut self, mode: BpmMode, now: Instant) {
        self.bpm_input = None;
        if self.bpm_mode == mode {
            return;
        }
        self.cancel_mouse_gesture();
        self.cancel_cycle_swap();
        let note_offs = self.state.take_reset_messages();
        self.send_scheduled(&note_offs);
        self.bpm_mode = mode;
        self.restart_timeline(now);
        crate::log_line(&format!(
            "grid-sequencer: bpm mode={} value={}",
            self.bpm_mode.label(),
            self.bpm()
        ));
    }
}

#[cfg(test)]
mod tests;
