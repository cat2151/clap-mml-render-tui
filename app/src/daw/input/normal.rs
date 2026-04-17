use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::{
    playback_util::effective_measure_count, AbRepeatState, DawApp, DawMode, DawNormalAction,
    DawPlayState, NormalPasteUndo, DEFAULT_TRACK0_MML, FIRST_PLAYABLE_TRACK,
};

const TEMPO_TRACK: usize = 0;
const INIT_MEASURE: usize = 0;

#[path = "normal/playback.rs"]
mod playback;

pub(super) use playback::{
    format_random_patch_hot_reload_log, normal_playback_shortcut, preview_target_tracks,
    resolve_playback_start_measure_index, NormalPlaybackShortcut,
};

impl DawApp {
    /// Applies the same random-patch update as pressing `r` on the target track.
    ///
    /// `Ok(false)` means no candidate patch was available, so the operation is a
    /// no-op. This matches the existing `r` key behavior, and HTTP callers also
    /// currently treat that case as a successful no-op.
    pub(in crate::daw) fn apply_random_patch_to_track(
        &mut self,
        track: usize,
    ) -> Result<bool, String> {
        if track < FIRST_PLAYABLE_TRACK {
            return Err("ランダム音色は演奏トラックでのみ使用できます".to_string());
        }
        let patch_filter_query = self.track_patch_filter_query(track);
        let Some(patch) = self.pick_random_patch_name_with_query(patch_filter_query.as_deref())
        else {
            return Ok(false);
        };
        let affected_measures: Vec<usize> = (1..=self.measures)
            .filter(|&measure| !self.data[track][measure].trim().is_empty())
            .collect();
        let current_init_mml = self.data[track][INIT_MEASURE].clone();
        self.data[track][INIT_MEASURE] = Self::replace_patch_name_in_mml(
            &current_init_mml,
            &patch,
            patch_filter_query.as_deref(),
        );
        self.invalidate_cell(track, INIT_MEASURE);
        self.invalidate_dependent_cells(track, INIT_MEASURE);
        self.start_track_rerender_batch(track, &affected_measures, "random patch update");
        self.save();

        let new_mmls = self.build_measure_mmls();
        let new_samples = self.measure_duration_samples();
        let old_effective_count = {
            let old_mmls = self.play_measure_mmls.lock().unwrap();
            effective_measure_count(&old_mmls)
        };
        let new_effective_count = effective_measure_count(&new_mmls);
        let old_samples = *self.play_measure_samples.lock().unwrap();
        let displayed_measure_index = self
            .play_position
            .lock()
            .unwrap()
            .as_ref()
            .map(|position| position.measure_index);
        self.append_log_line(format_random_patch_hot_reload_log(
            track,
            displayed_measure_index,
            old_effective_count,
            new_effective_count,
            old_samples,
            new_samples,
        ));
        self.sync_playback_mml_state();

        Ok(true)
    }

    fn apply_generate_to_current_measure(&mut self) {
        if self.cursor_track < FIRST_PLAYABLE_TRACK {
            self.append_log_line("generate は演奏トラックでのみ使用できます");
            return;
        }
        let Some(measure_index) = self.cursor_play_measure_index() else {
            self.append_log_line("generate は init 以外の小節でのみ使用できます");
            return;
        };
        let Some(patch_name) = self.pick_random_patch_name() else {
            return;
        };
        let generated_phrase = crate::generate::pick_default_generate_phrase();

        self.apply_generate_to_current_measure_with(patch_name, generated_phrase, measure_index);
    }

    pub(in crate::daw) fn apply_generate_to_current_measure_with(
        &mut self,
        patch_name: String,
        generated_phrase: &str,
        measure_index: usize,
    ) {
        let current = self.data[self.cursor_track][self.cursor_measure].clone();
        let next_patch_json = Self::build_patch_json(&patch_name);
        let init_changed = self.data[self.cursor_track][INIT_MEASURE] != next_patch_json;
        let measure_changed = current != generated_phrase;
        if !(init_changed || measure_changed) {
            return;
        }

        self.record_current_measure_to_patch_history(&current);
        if init_changed {
            self.commit_insert_cell(self.cursor_track, INIT_MEASURE, &next_patch_json);
        }
        if measure_changed {
            self.commit_insert_cell(self.cursor_track, self.cursor_measure, generated_phrase);
        }

        self.save();
        self.sync_playback_mml_state();
        self.stop_play();
        if self.try_start_preview_with_track_mmls_for_test(measure_index, None) {
            return;
        }
        self.start_preview(measure_index);
    }

    fn cut_current_measure(&mut self) {
        let current = self.data[self.cursor_track][self.cursor_measure].clone();
        self.record_current_measure_to_patch_history(&current);
        self.yank_buffer = Some(current);
        if self.commit_insert_cell(self.cursor_track, self.cursor_measure, "") {
            self.save();
            self.sync_playback_mml_state();
        }
    }

    fn paste_yanked_measure(&mut self) -> bool {
        let Some(yanked) = self.yank_buffer.as_deref() else {
            return false;
        };
        let yanked = yanked.to_string();
        let previous = self.data[self.cursor_track][self.cursor_measure].clone();
        self.record_current_measure_to_patch_history(&previous);
        if self.commit_insert_cell(self.cursor_track, self.cursor_measure, &yanked) {
            self.normal_paste_undo = Some(NormalPasteUndo {
                track: self.cursor_track,
                measure: self.cursor_measure,
                previous,
                pasted: yanked.clone(),
            });
            self.save();
            self.sync_playback_mml_state();
        }
        true
    }

    fn undo_last_paste(&mut self) -> bool {
        let Some(undo) = self.normal_paste_undo.take() else {
            return false;
        };
        if self.data[undo.track][undo.measure] != undo.pasted {
            return false;
        }
        if self.commit_insert_cell(undo.track, undo.measure, &undo.previous) {
            self.save();
            self.sync_playback_mml_state();
        }
        true
    }

    fn restore_default_tempo_init_if_empty(&mut self) -> bool {
        if self.cursor_track != TEMPO_TRACK
            || self.cursor_measure != INIT_MEASURE
            || !self.data[TEMPO_TRACK][INIT_MEASURE].trim().is_empty()
        {
            return false;
        }

        if self.commit_insert_cell(TEMPO_TRACK, INIT_MEASURE, DEFAULT_TRACK0_MML) {
            self.save();
            self.sync_playback_mml_state();
        }
        true
    }

    fn cycle_ab_repeat(&self) {
        let cursor_measure_index = self.cursor_play_measure_index();
        let mut ab_repeat = self.ab_repeat.lock().unwrap();
        *ab_repeat = match *ab_repeat {
            AbRepeatState::Off => cursor_measure_index
                .map(|cursor_measure_index| AbRepeatState::FixStart {
                    start_measure_index: cursor_measure_index,
                    end_measure_index: cursor_measure_index,
                })
                .unwrap_or(AbRepeatState::Off),
            AbRepeatState::FixStart {
                start_measure_index,
                end_measure_index,
            } => AbRepeatState::FixEnd {
                start_measure_index,
                end_measure_index: cursor_measure_index.unwrap_or(end_measure_index),
            },
            AbRepeatState::FixEnd { .. } => AbRepeatState::Off,
        };
    }

    fn start_preview_for_target_tracks(&mut self, preview_all_tracks: bool) {
        let play_state = *self.play_state.lock().unwrap();
        match play_state {
            DawPlayState::Idle => {}
            // カーソル移動に追従する preview は、現在の preview を止めて
            // 新しい対象に切り替える。一方で通常再生中は preview を開始しない。
            DawPlayState::Preview => self.stop_play(),
            DawPlayState::Playing => return,
        }
        let Some(measure_index) = self.cursor_play_measure_index() else {
            return;
        };
        let Some(target_tracks) =
            preview_target_tracks(self.tracks, self.cursor_track, preview_all_tracks)
        else {
            return;
        };
        if self.try_start_preview_for_test() {
            return;
        }
        self.start_preview_on_tracks(measure_index, &target_tracks);
    }

    fn toggle_preview_for_target_tracks(&mut self, preview_all_tracks: bool) {
        let play_state = *self.play_state.lock().unwrap();
        match play_state {
            DawPlayState::Idle => self.start_preview_for_target_tracks(preview_all_tracks),
            DawPlayState::Preview | DawPlayState::Playing => self.stop_play(),
        }
    }

    fn preview_current_target_if_stopped(&mut self) {
        let play_state = *self.play_state.lock().unwrap();
        if play_state == DawPlayState::Playing {
            return;
        }
        let is_previewable = self.cursor_play_measure_index().is_some()
            && self.cursor_track >= FIRST_PLAYABLE_TRACK
            && self.cursor_track < self.tracks;
        if !is_previewable {
            if play_state == DawPlayState::Preview {
                self.stop_play();
            }
            return;
        }
        if self.try_start_preview_for_test() {
            return;
        }
        self.start_preview_for_target_tracks(false);
    }

    // `new_for_test()` の DAW は PluginEntry を持たないため、
    // 実オーディオ preview を起動せず状態更新だけを検証する。
    #[cfg(test)]
    fn try_start_preview_for_test(&mut self) -> bool {
        let measure_index = self.cursor_play_measure_index().unwrap_or(0);
        self.try_start_preview_with_track_mmls_for_test(measure_index, None)
    }

    #[cfg(not(test))]
    fn try_start_preview_for_test(&mut self) -> bool {
        false
    }

    fn start_play_from_cursor_measure(&self) {
        if *self.play_state.lock().unwrap() != DawPlayState::Idle {
            return;
        }
        let Some(measure_index) = resolve_playback_start_measure_index(
            self.cursor_play_measure_index(),
            NormalPlaybackShortcut::PlayFromCursor,
        ) else {
            return;
        };
        self.start_play_from_measure(measure_index);
    }

    pub(in crate::daw) fn handle_normal_key_event(
        &mut self,
        key_event: KeyEvent,
    ) -> DawNormalAction {
        let is_plain_d_key =
            key_event.code == KeyCode::Char('d') && key_event.modifiers == KeyModifiers::NONE;
        if is_plain_d_key {
            if self.normal_pending_delete {
                self.normal_pending_delete = false;
                self.cut_current_measure();
            } else {
                self.normal_pending_delete = true;
            }
            return DawNormalAction::Continue;
        }
        self.normal_pending_delete = false;

        match normal_playback_shortcut(key_event) {
            Some(NormalPlaybackShortcut::PreviewCurrentTrack) => {
                self.toggle_preview_for_target_tracks(false);
                return DawNormalAction::Continue;
            }
            Some(NormalPlaybackShortcut::PreviewAllTracks) => {
                self.toggle_preview_for_target_tracks(true);
                return DawNormalAction::Continue;
            }
            Some(NormalPlaybackShortcut::PlayFromCursor) => {
                let play_state = *self.play_state.lock().unwrap();
                match play_state {
                    DawPlayState::Idle => self.start_play_from_cursor_measure(),
                    DawPlayState::Preview | DawPlayState::Playing => self.stop_play(),
                }
                return DawNormalAction::Continue;
            }
            Some(NormalPlaybackShortcut::TogglePlay) => {
                let state = *self.play_state.lock().unwrap();
                if state == DawPlayState::Playing || state == DawPlayState::Preview {
                    self.stop_play();
                } else {
                    self.start_play();
                }
                return DawNormalAction::Continue;
            }
            None => {}
        }

        match key_event.code {
            KeyCode::Char('q') => return DawNormalAction::QuitApp,
            KeyCode::Char('n') => return DawNormalAction::ReturnToTui,

            KeyCode::Char('h') | KeyCode::Left if self.cursor_measure > 0 => {
                self.cursor_measure -= 1;
                self.update_ab_repeat_follow_end_with_cursor();
                self.preview_current_target_if_stopped();
            }
            KeyCode::Char('H') => {
                self.start_history_overlay();
            }
            KeyCode::Char('l') | KeyCode::Right if self.cursor_measure < self.measures => {
                self.cursor_measure += 1;
                self.update_ab_repeat_follow_end_with_cursor();
                self.preview_current_target_if_stopped();
            }
            KeyCode::Char('j') | KeyCode::Down if self.cursor_track + 1 < self.tracks => {
                self.cursor_track += 1;
                self.preview_current_target_if_stopped();
            }
            KeyCode::Char('k') | KeyCode::Up if self.cursor_track > 0 => {
                self.cursor_track -= 1;
                self.preview_current_target_if_stopped();
            }
            KeyCode::Char('M') => {
                self.cursor_track = self.tracks / 2;
            }
            KeyCode::Char('L') => {
                self.cursor_track = self.tracks - 1;
            }

            KeyCode::Char('i') => self.start_insert(),
            KeyCode::Char('m') => {
                self.mixer_cursor_track = self
                    .cursor_track
                    .clamp(FIRST_PLAYABLE_TRACK, self.tracks - 1);
                self.mode = DawMode::Mixer;
            }

            KeyCode::Char('K') | KeyCode::Char('?') => self.enter_help(),

            KeyCode::Char('p') if !self.paste_yanked_measure() => {
                self.append_log_line("ヤンクバッファが空です".to_string());
            }
            KeyCode::Char('u') => {
                self.undo_last_paste();
            }

            KeyCode::Char('a') => self.cycle_ab_repeat(),

            KeyCode::Char('s') if self.cursor_track >= FIRST_PLAYABLE_TRACK => {
                if !self.solo_mode_active() {
                    self.solo_tracks.fill(false);
                    self.solo_tracks[self.cursor_track] = true;
                } else if let Some(is_solo) = self.solo_tracks.get_mut(self.cursor_track) {
                    *is_solo = !*is_solo;
                }
                self.sync_playback_mml_state();
            }

            KeyCode::Char('g') => self.apply_generate_to_current_measure(),
            KeyCode::Char('r') => {
                if self.restore_default_tempo_init_if_empty() {
                    return DawNormalAction::Continue;
                }
                if let Err(message) = self.apply_random_patch_to_track(self.cursor_track) {
                    self.append_log_line(message);
                }
            }

            _ => {}
        }
        DawNormalAction::Continue
    }

    #[cfg(test)]
    pub(super) fn handle_normal(&mut self, key: KeyCode) -> DawNormalAction {
        self.handle_normal_key_event(KeyEvent::new(key, KeyModifiers::NONE))
    }
}
