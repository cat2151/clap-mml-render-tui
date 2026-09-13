//! 画面が持つ状態を `cmrt-history` の素の値へ落として保存する（画面 → history）。
//!
//! 逆向き（history → 画面）は [`super::restore`]。

use super::grid_sequencer::grid_session_to_history;
use crate::tui::TuiApp;

/// 既定のままの範囲は保存しない（history.json にキーを増やさない）。
fn bpm_range_to_history(range: cmrt_tui_core::bpm::BpmRange, default_bpm: f64) -> Option<[f64; 2]> {
    (range != cmrt_tui_core::bpm::BpmRange::fixed(default_bpm))
        .then(|| [range.minimum(), range.maximum()])
}

/// [`super::restore::play_settings_from_history`] の逆。
fn play_settings_to_history(
    settings: crate::tui::mml_overlay::PlaySettings,
) -> crate::history::MmlOverlayPlaySettings {
    crate::history::MmlOverlayPlaySettings {
        repeat: settings.repeat,
        modulation: settings.filters.modulation,
        velocity: settings.filters.velocity,
    }
}

impl TuiApp<'_> {
    pub(in crate::tui) fn save_history_state(&self) {
        let _ = crate::history::save_session_state(&crate::history::SessionState {
            cursor: self.notepad.session_cursor(),
            lines: self.notepad.session_lines().to_vec(),
            active_screen: self.active_screen,
            keyboard: self.keyboard.state.session_state(),
            grid_sequencer_track_count: self.grid_sequencer.track_count(),
            grid_sequencer_chord_mode: self.grid_sequencer.chord_enabled(),
            grid_sequencer: grid_session_to_history(self.grid_sequencer.session_state()),
            grid_sequencer_bpm: self.grid_sequencer.bpm_mode().manual(),
            loop_browser_bpm: self.loop_browser.state.bpm_mode().manual(),
            grid_sequencer_bpm_range: bpm_range_to_history(
                self.grid_sequencer.bpm_range(),
                crate::tui::grid_sequencer::BPM,
            ),
            loop_browser_bpm_range: bpm_range_to_history(
                self.loop_browser.state.bpm_range(),
                crate::loop_browser::time_stretch::TARGET_BPM,
            ),
            keyboard_note_guide_overlay_date: self
                .keyboard
                .note_guide
                .last_overlay_date()
                .map(str::to_owned),
            notepad_sound_check_guide_overlay_date: self
                .notepad
                .sound_check_guide()
                .last_overlay_date()
                .map(str::to_owned),
            mml_overlay_patch: self.mml_overlay_patch.clone(),
            chord_chart_patch: self.chord_chart_patch.clone(),
            mml_overlay_play_settings: play_settings_to_history(self.mml_overlay.play_settings()),
        });
    }

    pub(in crate::tui) fn save_keyboard_note_guide_overlay_date(&self) {
        if let Some(local_date) = self.keyboard.note_guide.last_overlay_date() {
            let _ = crate::history::save_keyboard_note_guide_overlay_date(local_date);
        }
    }
}
