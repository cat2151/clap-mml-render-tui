use super::*;

impl LoopBrowser {
    pub(super) fn solo_mode_active(&self) -> bool {
        self.solo_tracks.iter().any(|&is_solo| is_solo)
    }

    pub(in crate::tui) fn track_is_soloed(&self, track: usize) -> bool {
        self.solo_tracks.get(track).copied().unwrap_or(false)
    }

    pub(in crate::tui) fn track_is_audible(&self, track: usize) -> bool {
        !self.solo_mode_active() || self.track_is_soloed(track)
    }

    pub(super) fn toggle_current_track_solo(&mut self) -> LoopBrowserAction {
        let track = self
            .track_cursor
            .min(self.track_grid.len().saturating_sub(1));
        self.solo_tracks.resize(self.track_grid.len(), false);
        if !self.solo_mode_active() {
            self.solo_tracks.fill(false);
            self.solo_tracks[track] = true;
        } else if let Some(is_solo) = self.solo_tracks.get_mut(track) {
            *is_solo = !*is_solo;
        }
        LoopBrowserAction::TrackSoloChanged {
            solo_tracks: self.solo_tracks.clone(),
        }
    }
}
