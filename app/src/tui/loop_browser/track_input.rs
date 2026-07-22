use super::*;

impl LoopBrowser {
    pub(super) fn normalize_track_grid(&mut self) {
        self.track_grid =
            crate::loop_browser::track_grid::normalize_previous_markers(&self.track_grid).0;
    }

    pub(super) fn save_track_grid(&self) -> anyhow::Result<()> {
        match &self.track_grid_path {
            Some(path) => crate::loop_browser::track_grid::save_to(
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
        let Some(wav) = self.metadata.value.pad(pad).cloned() else {
            return LoopBrowserAction::Continue;
        };
        self.log_pad_wav_context(pad, &wav);
        if !self.track_grid_writable {
            return LoopBrowserAction::Continue;
        }
        let occupied = self
            .clip_at(self.track_cursor, self.measure_cursor)
            .map(|(start, clip)| (start, clip.wav.clone(), clip.is_previous()));
        if occupied
            .as_ref()
            .is_some_and(|(_, _, is_previous)| *is_previous)
        {
            return match self.replace_current_clip(wav) {
                Some(start_measure) => {
                    self.sync_tree_to_current_cell();
                    LoopBrowserAction::GridReplaced {
                        start_measure,
                        grid: self.playback_grid(),
                        reason: LoopGridChange::Pad(pad),
                    }
                }
                None => LoopBrowserAction::Continue,
            };
        }
        if occupied
            .as_ref()
            .is_some_and(|(_, current, _)| !current.matches(&wav))
        {
            return match self.replace_current_clip(wav) {
                Some(start_measure) => {
                    self.sync_tree_to_current_cell();
                    LoopBrowserAction::GridReplaced {
                        start_measure,
                        grid: self.playback_grid(),
                        reason: LoopGridChange::Pad(pad),
                    }
                }
                None => LoopBrowserAction::Continue,
            };
        }
        if occupied.is_none() {
            return match self.insert_current_clip(wav) {
                Some(_) => {
                    self.sync_tree_to_current_cell();
                    LoopBrowserAction::GridRefresh {
                        grid: self.playback_grid(),
                        reason: LoopGridChange::Pad(pad),
                    }
                }
                None => LoopBrowserAction::Continue,
            };
        }
        let previous = self.track_grid.clone();
        if let Some((start, _, _)) = occupied {
            self.track_grid[self.track_cursor][start] = None;
        }
        self.normalize_track_grid();
        if let Err(error) = self.save_track_grid() {
            self.track_grid = previous;
            self.track_grid_error = Some(format!("track listを保存できません: {error}"));
            return LoopBrowserAction::Continue;
        }
        self.track_grid_error = None;
        self.sync_tree_to_current_cell();
        LoopBrowserAction::GridRefresh {
            grid: self.playback_grid(),
            reason: LoopGridChange::Pad(pad),
        }
    }

    fn log_pad_wav_context(&self, pad: char, wav: &LoopWavId) {
        let analysis = self.analysis_for_wav(wav);
        let declared_bpm = analysis
            .and_then(|value| value.tempo)
            .and_then(|tempo| tempo.declared_bpm)
            .map(crate::loop_browser::time_stretch::format_bpm)
            .unwrap_or_else(|| "none".to_string());
        let effective_bpm = analysis
            .and_then(|value| value.tempo)
            .map(|tempo| crate::loop_browser::time_stretch::format_bpm(tempo.bpm))
            .unwrap_or_else(|| "none".to_string());
        let favorite = self
            .metadata
            .value
            .deepest_favorite_for_wav(wav)
            .map_or("none", |(_, dir)| dir.relative.as_str());
        let category = self.category_for_wav(wav).unwrap_or("none");
        super::playback::diagnostics::log_event(format!(
            "event=pad-selected pad={} track={} measure={} path=\"{}\" favorite=\"{}\" category={} declared_bpm={} effective_bpm={}",
            pad,
            self.track_cursor + 1,
            self.measure_cursor + 1,
            super::playback::diagnostics::path_label(&wav.path()),
            favorite.replace('"', "\\\""),
            category,
            declared_bpm,
            effective_bpm,
        ));
    }

    pub(super) fn insert_current_clip(&mut self, wav: LoopWavId) -> Option<usize> {
        if !self.track_grid_writable
            || self
                .clip_at(self.track_cursor, self.measure_cursor)
                .is_some()
        {
            return None;
        }
        self.write_clip_at(self.measure_cursor, wav, "配置")
    }

    pub(super) fn replace_current_clip(&mut self, wav: LoopWavId) -> Option<usize> {
        if !self.track_grid_writable {
            return None;
        }
        let start = self
            .clip_at(self.track_cursor, self.measure_cursor)
            .map(|(start, _)| start)?;
        self.write_clip_at(start, wav, "差し替え")
    }

    fn write_clip_at(&mut self, start: usize, wav: LoopWavId, operation: &str) -> Option<usize> {
        let span_measures = self
            .analysis_for_wav(&wav)
            .map(|analysis| analysis.measures)
            .unwrap_or(1)
            .max(1);
        let Some(end) = start.checked_add(span_measures) else {
            self.track_grid_error = Some(format!(
                "track listが大きすぎるためclipを{operation}できません"
            ));
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
        self.track_grid[self.track_cursor][start] =
            Some(LoopTrackClip::explicit(wav, span_measures));
        self.normalize_track_grid();
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
        self.solo_tracks.resize(required, false);
        self.track_grid_error = None;
        self.track_cursor = next;
    }
}
