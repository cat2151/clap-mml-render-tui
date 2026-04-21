use super::super::{
    AbRepeatState, CacheState, CellCache, DawApp, DawPlayState, FIRST_PLAYABLE_TRACK, MIXER_MAX_DB,
    MIXER_MIN_DB,
};
use super::{
    current_state, take_pending_http_commands, DawHttpCommandKind, DawStatusCacheSnapshot,
    DawStatusGridSnapshot, DawStatusSnapshot,
};

impl DawApp {
    pub(in super::super) fn sync_http_grid_snapshot(&self) {
        let Some(state) = current_state() else {
            return;
        };
        let grid_snapshot = self.data.clone();
        state.lock().unwrap().grid_snapshot = grid_snapshot;
    }

    pub(in super::super) fn sync_http_status_snapshot(&self) {
        let Some(state) = current_state() else {
            return;
        };
        let play_state = *self.play_state.lock().unwrap();
        let play_position = self.play_position.lock().unwrap().clone();
        let ab_repeat = *self.ab_repeat.lock().unwrap();
        let beat_count = self.beat_numerator();
        let beat_duration_secs = 60.0 / self.tempo_bpm();
        let cache = self.cache.lock().unwrap();
        let mut pending_count = 0;
        let mut rendering_count = 0;
        let mut ready_count = 0;
        let mut error_count = 0;
        let cells = (0..self.tracks)
            .map(|track| {
                (0..=self.measures)
                    .map(|measure| {
                        let cache_state = cache[track][measure].state.clone();
                        match cache_state {
                            CacheState::Empty => {}
                            CacheState::Pending => pending_count += 1,
                            CacheState::Rendering => rendering_count += 1,
                            CacheState::Ready => ready_count += 1,
                            CacheState::Error => error_count += 1,
                        }
                        cache_state
                    })
                    .collect()
            })
            .collect();
        drop(cache);

        state.lock().unwrap().status_snapshot = Some(DawStatusSnapshot {
            play_state,
            play_position,
            beat_count,
            beat_duration_secs,
            ab_repeat,
            cache: DawStatusCacheSnapshot {
                cells,
                pending_count,
                rendering_count,
                ready_count,
                error_count,
            },
            grid: DawStatusGridSnapshot {
                tracks: self.tracks,
                measures: self.measures,
            },
        });
    }

    pub(in super::super) fn apply_pending_http_commands(&mut self) {
        for command in take_pending_http_commands() {
            let result = match command.kind {
                DawHttpCommandKind::Mml {
                    track,
                    measure,
                    mml,
                } => self.apply_http_mml(track, measure, &mml),
                DawHttpCommandKind::Mixer { track, db } => self.apply_http_mixer(track, db),
                DawHttpCommandKind::Patch { track, patch } => self.apply_http_patch(track, &patch),
                DawHttpCommandKind::RandomPatch { track } => self.apply_http_random_patch(track),
                DawHttpCommandKind::PlayStart => self.apply_http_play_start(),
                DawHttpCommandKind::PlayStop => self.apply_http_play_stop(),
                DawHttpCommandKind::AbRepeat {
                    start_measure,
                    end_measure,
                } => self.apply_http_ab_repeat(start_measure, end_measure),
            };
            self.sync_http_status_snapshot();
            let _ = command.response_tx.send(result);
        }
    }

    fn ensure_http_grid_size(&mut self, track: usize, measure: usize) -> Result<bool, String> {
        let required_tracks = track
            .checked_add(1)
            .ok_or_else(|| "track index が大きすぎます".to_string())?;
        let required_measures = self.measures.max(measure);
        let current_columns = self
            .measures
            .checked_add(1)
            .ok_or_else(|| "現在の measure 数が大きすぎます".to_string())?;
        let required_columns = required_measures
            .checked_add(1)
            .ok_or_else(|| "measure index が大きすぎます".to_string())?;
        if required_tracks <= self.tracks && required_measures <= self.measures {
            return Ok(false);
        }
        let mut resized = false;

        if required_tracks > self.tracks {
            resized = true;
            self.data.resize_with(required_tracks, || {
                let mut row = Vec::new();
                row.resize_with(current_columns, String::new);
                row
            });
            {
                let mut cache = self.cache.lock().unwrap();
                cache.resize_with(required_tracks, || {
                    vec![CellCache::empty(); current_columns]
                });
            }
            self.solo_tracks.resize(required_tracks, false);
            self.track_volumes_db.resize(required_tracks, 0);
            self.play_track_gains
                .lock()
                .unwrap()
                .resize(required_tracks, 0.0);
            self.track_rerender_batches
                .lock()
                .unwrap()
                .resize(required_tracks, None);
            self.tracks = required_tracks;
        }

        if required_measures > self.measures {
            resized = true;
            for row in &mut self.data {
                row.resize_with(required_columns, String::new);
            }
            {
                let mut cache = self.cache.lock().unwrap();
                for row in cache.iter_mut() {
                    row.resize_with(required_columns, CellCache::empty);
                }
            }
            self.play_measure_mmls
                .lock()
                .unwrap()
                .resize_with(required_measures, String::new);
            self.play_measure_track_mmls
                .lock()
                .unwrap()
                .resize_with(required_measures, || vec![String::new(); self.tracks]);
            self.measures = required_measures;
        }

        for measure_track_mmls in self.play_measure_track_mmls.lock().unwrap().iter_mut() {
            measure_track_mmls.resize_with(self.tracks, String::new);
        }

        Ok(resized)
    }

    pub(in super::super) fn apply_http_mml(
        &mut self,
        track: usize,
        measure: usize,
        mml: &str,
    ) -> Result<(), String> {
        if measure == 0 {
            return Err("measure は 1 以上を指定してください".to_string());
        }
        self.ensure_http_grid_size(track, measure)?;
        self.commit_insert_cell(track, measure, mml);
        self.save();
        self.sync_playback_mml_state();
        self.append_log_line(format!("http: mml track={track} meas={measure}"));
        Ok(())
    }

    fn apply_http_mixer(&mut self, track: usize, db: f64) -> Result<(), String> {
        if !db.is_finite() {
            return Err("db は有限な数値を指定してください".to_string());
        }
        if track < FIRST_PLAYABLE_TRACK {
            return Err("mixer は演奏トラックでのみ使用できます".to_string());
        }
        let grid_resized = self.ensure_http_grid_size(track, self.measures.max(1))?;
        if grid_resized {
            self.sync_http_grid_snapshot();
        }

        let rounded_db = db.round() as i32;
        let clamped_db = rounded_db.clamp(MIXER_MIN_DB, MIXER_MAX_DB);
        let current_db = self.track_volume_db(track);
        if clamped_db != current_db {
            let _ = self.adjust_track_volume_db(track, clamped_db - current_db);
            self.save();
            self.sync_playback_mml_state();
        }
        self.append_log_line(format!("http: mixer track={track} db={clamped_db}"));
        Ok(())
    }

    fn apply_http_patch(&mut self, track: usize, patch_name: &str) -> Result<(), String> {
        if track < FIRST_PLAYABLE_TRACK {
            return Err("patch は演奏トラックでのみ使用できます".to_string());
        }
        self.ensure_http_grid_size(track, self.measures.max(1))?;

        let patch_pairs = crate::patches::collect_patch_pairs(self.cfg.as_ref())
            .map_err(|error| format!("patch 一覧の取得に失敗しました: {error}"))?;
        let display_patch_name =
            crate::patches::resolve_display_patch_name(&patch_pairs, patch_name)
                .ok_or_else(|| format!("patch が見つかりません: {patch_name}"))?;
        let patch_json = Self::build_patch_json(&display_patch_name);
        self.commit_insert_cell(track, 0, &patch_json);
        self.save();
        self.sync_playback_mml_state();
        self.append_log_line(format!(
            "http: patch track={track} patch={display_patch_name}"
        ));
        Ok(())
    }

    fn apply_http_random_patch(&mut self, track: usize) -> Result<(), String> {
        self.ensure_http_grid_size(track, self.measures.max(1))?;
        self.apply_random_patch_to_track(track)?;
        self.sync_http_grid_snapshot();
        self.append_log_line(format!("http: patch/random track={track}"));
        Ok(())
    }

    fn apply_http_play_start(&mut self) -> Result<(), String> {
        let play_state = *self.play_state.lock().unwrap();
        if play_state == DawPlayState::Playing {
            self.append_log_line("http: play start (already playing)");
            return Ok(());
        }
        if play_state == DawPlayState::Preview {
            self.stop_play();
        }
        self.start_play();
        if *self.play_state.lock().unwrap() == DawPlayState::Playing {
            self.append_log_line("http: play start");
            Ok(())
        } else {
            self.append_log_line("http: play start (no playable data)");
            Err("再生可能なデータがありません".to_string())
        }
    }

    fn apply_http_play_stop(&mut self) -> Result<(), String> {
        self.stop_play();
        self.append_log_line("http: play stop");
        Ok(())
    }

    pub(in super::super) fn apply_http_ab_repeat(
        &mut self,
        start_measure: usize,
        end_measure: usize,
    ) -> Result<(), String> {
        if start_measure == 0 || end_measure == 0 {
            return Err("measA と measB は 1 以上を指定してください".to_string());
        }
        if start_measure > self.measures || end_measure > self.measures {
            return Err(format!(
                "measA と measB は 1..={} の範囲で指定してください",
                self.measures
            ));
        }

        *self.ab_repeat.lock().unwrap() = AbRepeatState::FixEnd {
            start_measure_index: start_measure - 1,
            end_measure_index: end_measure - 1,
        };
        self.append_log_line(format!(
            "http: ab-repeat measA={start_measure} measB={end_measure}"
        ));
        Ok(())
    }
}
