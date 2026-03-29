use super::{DawApp, FIRST_PLAYABLE_TRACK, MIXER_MAX_DB, MIXER_MIN_DB};

impl DawApp {
    pub(super) fn solo_mode_active(&self) -> bool {
        self.solo_tracks
            .iter()
            .enumerate()
            .skip(FIRST_PLAYABLE_TRACK)
            .any(|(_, &is_solo)| is_solo)
    }

    pub(super) fn track_is_soloed(&self, track: usize) -> bool {
        self.solo_tracks.get(track).copied().unwrap_or(false)
    }

    pub(super) fn track_is_audible(&self, track: usize) -> bool {
        if track < FIRST_PLAYABLE_TRACK || !self.solo_mode_active() {
            return true;
        }
        self.track_is_soloed(track)
    }

    pub(super) fn track_volume_db(&self, track: usize) -> i32 {
        self.track_volumes_db.get(track).copied().unwrap_or(0)
    }

    pub(super) fn adjust_track_volume_db(&mut self, track: usize, delta_db: i32) -> bool {
        let Some(volume_db) = self.track_volumes_db.get_mut(track) else {
            return false;
        };
        let next = (*volume_db + delta_db).clamp(MIXER_MIN_DB, MIXER_MAX_DB);
        if next == *volume_db {
            return false;
        }
        *volume_db = next;
        true
    }

    pub(super) fn playback_track_gains(&self) -> Vec<f32> {
        (0..self.tracks)
            .map(|track| {
                if track < FIRST_PLAYABLE_TRACK || !self.track_is_audible(track) {
                    0.0
                } else {
                    10.0f32.powf(self.track_volume_db(track) as f32 / 20.0)
                }
            })
            .collect()
    }
}
