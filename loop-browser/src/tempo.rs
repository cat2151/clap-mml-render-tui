use cmrt_tui_core::bpm::{BpmInputAction, BpmMode};
use crossterm::event::KeyEvent;

use crate::{LoopBrowser, LoopBrowserAction};

impl LoopBrowser {
    pub(crate) fn handle_bpm_input_key(&mut self, key: KeyEvent) -> LoopBrowserAction {
        let Some(input) = self.bpm_input.as_mut() else {
            return LoopBrowserAction::Continue;
        };
        match input.handle_key(key) {
            BpmInputAction::Continue => LoopBrowserAction::Continue,
            BpmInputAction::Cancel => {
                self.bpm_input = None;
                LoopBrowserAction::Continue
            }
            BpmInputAction::Apply(mode) => self.apply_bpm_mode(mode),
            BpmInputAction::ApplyAuto(range) => {
                let range_changed = range.is_some_and(|range| {
                    let changed = self.bpm_range != range;
                    self.bpm_range = range;
                    changed
                });
                self.apply_bpm_change(BpmMode::Auto(self.bpm_range.sample()), range_changed)
            }
        }
    }

    fn apply_bpm_mode(&mut self, mode: BpmMode) -> LoopBrowserAction {
        self.apply_bpm_change(mode, false)
    }

    fn apply_bpm_change(&mut self, mode: BpmMode, range_changed: bool) -> LoopBrowserAction {
        self.bpm_input = None;
        if self.bpm_mode == mode && !range_changed {
            return LoopBrowserAction::Continue;
        }
        self.bpm_mode = mode;
        self.auto_random_staged = None;
        self.normalize_track_grid();
        if let Err(error) = self.save_track_grid() {
            self.track_grid_error = Some(format!("BPM変更後のtrack listを保存できません: {error}"));
        }
        LoopBrowserAction::BpmChanged {
            mode,
            grid: self.playback_grid(),
        }
    }
}

#[cfg(test)]
mod tests;
