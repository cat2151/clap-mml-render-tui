use crossterm::event::KeyCode;

use super::super::{DawApp, DawMode, FIRST_PLAYABLE_TRACK};

impl DawApp {
    pub(in crate::daw) fn handle_help(&mut self, key: KeyCode) {
        if key == KeyCode::Esc {
            self.mode = DawMode::Normal;
        }
    }

    pub(in crate::daw) fn handle_mixer(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.mode = DawMode::Normal;
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if self.mixer_cursor_track > FIRST_PLAYABLE_TRACK {
                    self.mixer_cursor_track -= 1;
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.mixer_cursor_track + 1 < self.tracks {
                    self.mixer_cursor_track += 1;
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.adjust_track_volume_db(self.mixer_cursor_track, -3) {
                    self.save();
                    self.sync_playback_mml_state();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.adjust_track_volume_db(self.mixer_cursor_track, 3) {
                    self.save();
                    self.sync_playback_mml_state();
                }
            }
            _ => {}
        }
    }
}
