use super::*;
use rand::seq::SliceRandom;

impl LoopBrowser {
    pub fn solo_mode_active(&self) -> bool {
        self.solo_tracks.iter().any(|&is_solo| is_solo)
    }

    pub fn track_is_soloed(&self, track: usize) -> bool {
        self.solo_tracks.get(track).copied().unwrap_or(false)
    }

    pub fn track_is_audible(&self, track: usize) -> bool {
        !self.solo_mode_active() || self.track_is_soloed(track)
    }

    pub fn toggle_current_track_solo(&mut self) -> LoopBrowserAction {
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

    pub fn randomize_track_solos(&mut self) -> LoopBrowserAction {
        let mut eligible = self
            .track_grid
            .iter()
            .enumerate()
            .filter_map(|(track, cells)| {
                cells
                    .iter()
                    .filter_map(Option::as_ref)
                    .any(|clip| !clip.is_previous())
                    .then_some(track)
            })
            .collect::<Vec<_>>();
        eligible.shuffle(&mut rand::thread_rng());
        self.solo_tracks.resize(self.track_grid.len(), false);
        self.solo_tracks.fill(false);
        for track in eligible.into_iter().take(2) {
            self.solo_tracks[track] = true;
        }
        LoopBrowserAction::TrackSoloChanged {
            solo_tracks: self.solo_tracks.clone(),
        }
    }
}
