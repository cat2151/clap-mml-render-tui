use std::time::Instant;

use super::guide;
use crate::tui::TuiApp;

impl TuiApp<'_> {
    pub(in crate::tui) fn pump_keyboard_periodic(&mut self) {
        self.sync_keyboard_voicing_detection();
        let ready = self.keyboard_connection_status().phase.accepts_notes();
        let now = Instant::now();
        let first_overlay_today =
            self.keyboard
                .note_guide
                .tick(now, ready, &guide::local_date_string());
        if first_overlay_today {
            self.save_keyboard_note_guide_overlay_date();
        }
        if !ready {
            return;
        }
        // patch切替後の現在値再送(refresh) → 周期送信、の順で1回のsendにまとめる
        let mut messages = self.keyboard.state.take_pending_refresh_messages(now);
        messages.extend(self.keyboard.state.poll_periodic(now));
        if !messages.is_empty() {
            if let Some(sender) = &self.keyboard.midi_sender {
                sender.send(messages, self.keyboard.state.patch());
            }
        }
    }
}
