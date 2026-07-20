use super::*;

impl LoopBrowser {
    fn save_track_grid(&self) -> anyhow::Result<()> {
        match &self.track_grid_path {
            Some(path) => crate::loop_browser_track_grid::save_to(
                path,
                &self.track_grid,
                &self.track_volumes_db,
            ),
            None => Ok(()),
        }
    }

    pub(super) fn adjust_mixer_volume(&mut self, delta_db: i32) -> LoopBrowserAction {
        if !self.track_grid_writable {
            return LoopBrowserAction::Continue;
        }
        let track = self.mixer_cursor_track;
        let Some(volume_db) = self.track_volumes_db.get_mut(track) else {
            return LoopBrowserAction::Continue;
        };
        let previous = *volume_db;
        if !crate::mixer_overlay::adjust_volume_db(volume_db, delta_db) {
            return LoopBrowserAction::Continue;
        }
        let next = *volume_db;
        if let Err(error) = self.save_track_grid() {
            self.track_volumes_db[track] = previous;
            self.track_grid_error = Some(format!("mix levelを保存できません: {error}"));
            return LoopBrowserAction::Continue;
        }
        self.track_grid_error = None;
        LoopBrowserAction::TrackVolumeChanged {
            track,
            volume_db: next,
        }
    }

    pub(super) fn toggle_current_cell(&mut self, pad: char) -> LoopBrowserAction {
        let Some(wav) = self.metadata.pad(pad).cloned() else {
            return LoopBrowserAction::Continue;
        };
        if !self.track_grid_writable {
            return LoopBrowserAction::Continue;
        }
        let occupied = self
            .clip_at(self.track_cursor, self.measure_cursor)
            .map(|(start, clip)| (start, clip.wav.clone()));
        if occupied
            .as_ref()
            .is_some_and(|(_, current)| !current.matches(&wav))
        {
            return match self.replace_current_clip(wav) {
                Some(start_measure) => LoopBrowserAction::GridReplaced {
                    start_measure,
                    grid: self.playback_grid(),
                },
                None => LoopBrowserAction::Continue,
            };
        }
        let previous = self.track_grid.clone();
        if let Some((start, _)) = occupied.filter(|(_, current)| current.matches(&wav)) {
            self.track_grid[self.track_cursor][start] = None;
        } else {
            let span_measures = self
                .analysis_for_wav(&wav)
                .map(|analysis| analysis.measures)
                .unwrap_or(1)
                .max(1);
            let Some(end) = self.measure_cursor.checked_add(span_measures) else {
                self.track_grid_error =
                    Some("track listが大きすぎるためclipを配置できません".to_string());
                return LoopBrowserAction::Continue;
            };
            if end > self.track_grid[0].len() {
                for track in &mut self.track_grid {
                    track.resize(end, None);
                }
            }
            for measure in 0..self.track_grid[self.track_cursor].len() {
                let overlaps = self.track_grid[self.track_cursor][measure]
                    .as_ref()
                    .is_some_and(|clip| {
                        measure < end
                            && measure.saturating_add(clip.span_measures) > self.measure_cursor
                    });
                if overlaps {
                    self.track_grid[self.track_cursor][measure] = None;
                }
            }
            self.track_grid[self.track_cursor][self.measure_cursor] =
                Some(LoopTrackClip { wav, span_measures });
        }
        if let Err(error) = self.save_track_grid() {
            self.track_grid = previous;
            self.track_grid_error = Some(format!("track listを保存できません: {error}"));
            return LoopBrowserAction::Continue;
        }
        self.track_grid_error = None;
        LoopBrowserAction::GridRefresh(self.playback_grid())
    }

    pub(super) fn replace_current_clip(&mut self, wav: LoopWavId) -> Option<usize> {
        if !self.track_grid_writable {
            return None;
        }
        let start = self
            .clip_at(self.track_cursor, self.measure_cursor)
            .map(|(start, _)| start)?;
        let span_measures = self
            .analysis_for_wav(&wav)
            .map(|analysis| analysis.measures)
            .unwrap_or(1)
            .max(1);
        let Some(end) = start.checked_add(span_measures) else {
            self.track_grid_error =
                Some("track listが大きすぎるためclipを差し替えできません".to_string());
            return None;
        };
        let previous = self.track_grid.clone();
        if end > self.track_grid[0].len() {
            for track in &mut self.track_grid {
                track.resize(end, None);
            }
        }
        for measure in 0..self.track_grid[self.track_cursor].len() {
            let overlaps = self.track_grid[self.track_cursor][measure]
                .as_ref()
                .is_some_and(|clip| {
                    measure < end && measure.saturating_add(clip.span_measures) > start
                });
            if overlaps {
                self.track_grid[self.track_cursor][measure] = None;
            }
        }
        self.track_grid[self.track_cursor][start] = Some(LoopTrackClip { wav, span_measures });
        if let Err(error) = self.save_track_grid() {
            self.track_grid = previous;
            self.track_grid_error = Some(format!("track listを保存できません: {error}"));
            return None;
        }
        self.track_grid_error = None;
        Some(start)
    }

    pub(super) fn move_track_cursor_right(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        let measures = self.track_grid[0].len();
        let next = self.measure_cursor.saturating_add(count);
        if next < measures {
            self.measure_cursor = next;
            return;
        }
        if !self.track_grid_writable {
            self.measure_cursor = measures.saturating_sub(1);
            return;
        }
        let Some(required) = next.checked_add(1) else {
            self.track_grid_error =
                Some("track listが大きすぎるためmeasureを追加できません".to_string());
            return;
        };
        let mut updated = self.track_grid.clone();
        for track in &mut updated {
            if track.try_reserve_exact(required - track.len()).is_err() {
                self.track_grid_error =
                    Some("track listが大きすぎるためmeasureを追加できません".to_string());
                return;
            }
            track.resize(required, None);
        }
        let previous = std::mem::replace(&mut self.track_grid, updated);
        if let Err(error) = self.save_track_grid() {
            self.track_grid = previous;
            self.track_grid_error = Some(format!("measure追加を保存できません: {error}"));
            return;
        }
        self.track_grid_error = None;
        self.measure_cursor = next;
    }

    pub(super) fn move_track_cursor_down(&mut self, count: usize) {
        if count == 0 {
            return;
        }
        let next = self.track_cursor.saturating_add(count);
        if next < self.track_grid.len() {
            self.track_cursor = next;
            return;
        }
        if !self.track_grid_writable {
            self.track_cursor = self.track_grid.len().saturating_sub(1);
            return;
        }
        let Some(required) = next.checked_add(1) else {
            self.track_grid_error =
                Some("track listが大きすぎるためtrackを追加できません".to_string());
            return;
        };
        let mut updated = self.track_grid.clone();
        if updated.try_reserve_exact(required - updated.len()).is_err() {
            self.track_grid_error =
                Some("track listが大きすぎるためtrackを追加できません".to_string());
            return;
        }
        let measures = self.track_grid[0].len();
        updated.resize_with(required, || vec![None; measures]);
        let previous = std::mem::replace(&mut self.track_grid, updated);
        let previous_volumes = self.track_volumes_db.clone();
        self.track_volumes_db.resize(required, 0);
        if let Err(error) = self.save_track_grid() {
            self.track_grid = previous;
            self.track_volumes_db = previous_volumes;
            self.track_grid_error = Some(format!("track追加を保存できません: {error}"));
            return;
        }
        self.track_grid_error = None;
        self.track_cursor = next;
    }
}
