use super::super::{
    CellCache, DawApp, DawPlayState, FIRST_PLAYABLE_TRACK, MIXER_MAX_DB, MIXER_MIN_DB,
};

impl DawApp {
    pub(super) fn ensure_http_grid_size(
        &mut self,
        track: usize,
        measure: usize,
    ) -> Result<bool, String> {
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

    pub(super) fn apply_http_mixer(&mut self, track: usize, db: f64) -> Result<(), String> {
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

    pub(super) fn apply_http_patch(
        &mut self,
        track: usize,
        patch_name: &str,
    ) -> Result<(), String> {
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

    pub(super) fn apply_http_random_patch(&mut self, track: usize) -> Result<(), String> {
        self.ensure_http_grid_size(track, self.measures.max(1))?;
        self.apply_random_patch_to_track(track)?;
        self.sync_http_grid_snapshot();
        self.append_log_line(format!("http: patch/random track={track}"));
        Ok(())
    }

    pub(super) fn apply_http_play_start(&mut self) -> Result<(), String> {
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

    pub(super) fn apply_http_play_stop(&mut self) -> Result<(), String> {
        self.stop_play();
        self.append_log_line("http: play stop");
        Ok(())
    }
}
