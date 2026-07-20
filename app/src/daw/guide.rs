use std::time::Instant;

use super::{DawApp, DawMode};

impl DawApp {
    pub(super) fn pump_sound_check_guide(&mut self) {
        let first_overlay_today = self.sound_check_guide.tick(
            Instant::now(),
            self.mode == DawMode::Normal,
            &crate::sound_check_guide::local_date_string(),
        );
        if first_overlay_today {
            if let Some(local_date) = self.sound_check_guide.last_overlay_date() {
                let _ = crate::history::save_daw_sound_check_guide_overlay_date(local_date);
            }
        }
    }
}
