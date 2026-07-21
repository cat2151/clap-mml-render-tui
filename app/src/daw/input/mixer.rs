use crossterm::event::KeyCode;

use super::super::{DawApp, DawMode, FIRST_PLAYABLE_TRACK};

impl DawApp {
    pub(in crate::daw) fn handle_help(&mut self, key: KeyCode) {
        if key == KeyCode::Esc {
            self.mode = self.help_origin;
        }
    }

    pub(in crate::daw) fn handle_mixer(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.mode = DawMode::Normal;
            }
            KeyCode::Char('h') | KeyCode::Left
                if self.overlays.mixer.cursor_track > FIRST_PLAYABLE_TRACK =>
            {
                self.overlays.mixer.cursor_track -= 1;
            }
            KeyCode::Char('l') | KeyCode::Right
                if self.overlays.mixer.cursor_track + 1 < self.tracks =>
            {
                self.overlays.mixer.cursor_track += 1;
            }
            KeyCode::Char('j') | KeyCode::Down
                if self.adjust_track_volume_db(
                    self.overlays.mixer.cursor_track,
                    -crate::mixer_overlay::MIXER_STEP_DB,
                ) =>
            {
                self.save();
                self.sync_playback_mml_state();
            }
            KeyCode::Char('k') | KeyCode::Up
                if self.adjust_track_volume_db(
                    self.overlays.mixer.cursor_track,
                    crate::mixer_overlay::MIXER_STEP_DB,
                ) =>
            {
                self.save();
                self.sync_playback_mml_state();
            }
            _ => {}
        }
    }
}
