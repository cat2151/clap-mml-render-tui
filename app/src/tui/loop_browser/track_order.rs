use super::*;

impl LoopBrowser {
    pub(super) fn move_current_track(&mut self, delta: isize) -> LoopBrowserAction {
        if !self.track_grid_writable || self.track_grid.len() < 2 {
            return LoopBrowserAction::Continue;
        }
        let Some(destination) = self.track_cursor.checked_add_signed(delta) else {
            return LoopBrowserAction::Continue;
        };
        if destination >= self.track_grid.len() {
            return LoopBrowserAction::Continue;
        }

        self.track_volumes_db.resize(self.track_grid.len(), 0);
        self.solo_tracks.resize(self.track_grid.len(), false);
        self.track_grid.swap(self.track_cursor, destination);
        self.track_volumes_db.swap(self.track_cursor, destination);
        self.solo_tracks.swap(self.track_cursor, destination);

        if let Err(error) = self.save_track_grid() {
            self.track_grid.swap(self.track_cursor, destination);
            self.track_volumes_db.swap(self.track_cursor, destination);
            self.solo_tracks.swap(self.track_cursor, destination);
            self.track_grid_error = Some(format!("track並び替えを保存できません: {error}"));
            return LoopBrowserAction::Continue;
        }

        self.track_grid_error = None;
        self.track_cursor = destination;
        self.sync_tree_to_current_cell();
        LoopBrowserAction::TrackLayoutChanged {
            start_measure: self.measure_cursor,
            grid: self.playback_grid(),
            track_volumes_db: self.track_volumes_db.clone(),
            solo_tracks: self.solo_tracks.clone(),
        }
    }
}
