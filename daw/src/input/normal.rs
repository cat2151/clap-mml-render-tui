use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use super::super::{
    AbRepeatState, DawApp, DawMode, DawNormalAction, DawPlayState, NormalPasteUndo,
    DEFAULT_TRACK0_MML, FIRST_PLAYABLE_TRACK,
};
use super::track_patch::PatchUpdateReason;

const TEMPO_TRACK: usize = 0;
const INIT_MEASURE: usize = 0;

mod playback;

pub(super) use playback::{
    format_patch_hot_reload_log, normal_playback_shortcut, preview_target_tracks,
    resolve_playback_start_measure_index, NormalPlaybackShortcut,
};

impl DawApp {
    /// Applies the same random-patch update as pressing `r` on the target track.
    ///
    /// `Ok(false)` means no candidate patch was available, so the operation is a
    /// no-op. This matches the existing `r` key behavior, and HTTP callers also
    /// currently treat that case as a successful no-op.
    pub(crate) fn apply_random_patch_to_track(&mut self, track: usize) -> Result<bool, String> {
        if track < FIRST_PLAYABLE_TRACK {
            return Err("ランダム音色は演奏トラックでのみ使用できます".to_string());
        }
        if track >= self.editor.tracks {
            return Err(format!(
                "track は {}..={} の範囲で指定してください",
                FIRST_PLAYABLE_TRACK,
                self.editor.tracks.saturating_sub(1)
            ));
        }
        let patch_filter_query = self.track_patch_filter_query(track);
        let Some(patch) = self.pick_random_patch_name_with_query(patch_filter_query.as_deref())
        else {
            return Ok(false);
        };
        self.apply_patch_name_to_track_init(
            track,
            &patch,
            patch_filter_query.as_deref(),
            PatchUpdateReason::RandomPatch,
        );

        Ok(true)
    }

    fn apply_generate_to_current_measure(&mut self) {
        if self.editor.cursor_track < FIRST_PLAYABLE_TRACK {
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
        let generated_phrase = cmrt_tui_core::generate::pick_default_generate_phrase();

        self.apply_generate_to_current_measure_with(patch_name, generated_phrase, measure_index);
    }

    pub(crate) fn apply_generate_to_current_measure_with(
        &mut self,
        patch_name: String,
        generated_phrase: &str,
        measure_index: usize,
    ) {
        let current =
            self.editor.data[self.editor.cursor_track][self.editor.cursor_measure].clone();
        let next_patch_json = Self::build_patch_json(&patch_name);
        let init_changed =
            self.editor.data[self.editor.cursor_track][INIT_MEASURE] != next_patch_json;
        let measure_changed = current != generated_phrase;
        if !(init_changed || measure_changed) {
            return;
        }

        self.record_current_measure_to_patch_history(&current);
        if init_changed {
            self.commit_insert_cell(self.editor.cursor_track, INIT_MEASURE, &next_patch_json);
        }
        if measure_changed {
            self.commit_insert_cell(
                self.editor.cursor_track,
                self.editor.cursor_measure,
                generated_phrase,
            );
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
        let current =
            self.editor.data[self.editor.cursor_track][self.editor.cursor_measure].clone();
        self.record_current_measure_to_patch_history(&current);
        self.editor.yank_buffer = Some(current);
        if self.commit_insert_cell(self.editor.cursor_track, self.editor.cursor_measure, "") {
            self.save();
            self.sync_playback_mml_state();
        }
    }

    fn paste_yanked_measure(&mut self) -> bool {
        let Some(yanked) = self.editor.yank_buffer.as_deref() else {
            return false;
        };
        let yanked = yanked.to_string();
        let previous =
            self.editor.data[self.editor.cursor_track][self.editor.cursor_measure].clone();
        self.record_current_measure_to_patch_history(&previous);
        if self.commit_insert_cell(
            self.editor.cursor_track,
            self.editor.cursor_measure,
            &yanked,
        ) {
            self.editor.paste_undo = Some(NormalPasteUndo {
                track: self.editor.cursor_track,
                measure: self.editor.cursor_measure,
                previous,
                pasted: yanked.clone(),
            });
            self.save();
            self.sync_playback_mml_state();
        }
        true
    }

    fn undo_last_paste(&mut self) -> bool {
        let Some(undo) = self.editor.paste_undo.take() else {
            return false;
        };
        if self.editor.data[undo.track][undo.measure] != undo.pasted {
            return false;
        }
        if self.commit_insert_cell(undo.track, undo.measure, &undo.previous) {
            self.save();
            self.sync_playback_mml_state();
        }
        true
    }

    fn restore_default_tempo_init_if_empty(&mut self) -> bool {
        if self.editor.cursor_track != TEMPO_TRACK
            || self.editor.cursor_measure != INIT_MEASURE
            || !self.editor.data[TEMPO_TRACK][INIT_MEASURE]
                .trim()
                .is_empty()
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
        let mut ab_repeat = self.playback.ab_repeat.lock().unwrap();
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
        let play_state = *self.playback.play_state.lock().unwrap();
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
        let Some(target_tracks) = preview_target_tracks(
            self.editor.tracks,
            self.editor.cursor_track,
            preview_all_tracks,
        ) else {
            return;
        };
        if self.try_start_preview_for_test() {
            return;
        }
        self.start_preview_on_tracks(measure_index, &target_tracks);
    }

    fn toggle_preview_for_target_tracks(&mut self, preview_all_tracks: bool) {
        let play_state = *self.playback.play_state.lock().unwrap();
        match play_state {
            DawPlayState::Idle => self.start_preview_for_target_tracks(preview_all_tracks),
            DawPlayState::Preview | DawPlayState::Playing => self.stop_play(),
        }
    }

    fn preview_current_target_if_stopped(&mut self) {
        let play_state = *self.playback.play_state.lock().unwrap();
        if play_state == DawPlayState::Playing {
            return;
        }
        let is_previewable = self.cursor_play_measure_index().is_some()
            && self.editor.cursor_track >= FIRST_PLAYABLE_TRACK
            && self.editor.cursor_track < self.editor.tracks;
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
        if *self.playback.play_state.lock().unwrap() != DawPlayState::Idle {
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

    pub(crate) fn handle_normal_key_event(&mut self, key_event: KeyEvent) -> DawNormalAction {
        if key_event.modifiers == KeyModifiers::NONE
            && matches!(key_event.code, KeyCode::Char('h' | 'j' | 'k' | 'l'))
        {
            self.sound_check_guide.complete();
        }
        let is_plain_d_key =
            key_event.code == KeyCode::Char('d') && key_event.modifiers == KeyModifiers::NONE;
        if is_plain_d_key {
            if self.editor.pending_delete {
                self.editor.pending_delete = false;
                self.cut_current_measure();
            } else {
                self.editor.pending_delete = true;
            }
            return DawNormalAction::Continue;
        }
        self.editor.pending_delete = false;

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
                let play_state = *self.playback.play_state.lock().unwrap();
                match play_state {
                    DawPlayState::Idle => self.start_play_from_cursor_measure(),
                    DawPlayState::Preview | DawPlayState::Playing => self.stop_play(),
                }
                return DawNormalAction::Continue;
            }
            Some(NormalPlaybackShortcut::TogglePlay) => {
                let state = *self.playback.play_state.lock().unwrap();
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
            KeyCode::Char('n') => {
                return DawNormalAction::SwitchTo(
                    cmrt_tui_core::screen_switch::PrimaryScreen::Notepad,
                )
            }
            KeyCode::Char('v') => {
                return DawNormalAction::SwitchTo(
                    cmrt_tui_core::screen_switch::PrimaryScreen::Keyboard,
                )
            }
            KeyCode::Char('e') => return DawNormalAction::EditConfig,
            KeyCode::Char('f') if self.accepts_project_file_key(key_event.modifiers) => {
                self.start_project_overlay()
            }

            KeyCode::Char('h') | KeyCode::Left if self.editor.cursor_measure > 0 => {
                self.editor.cursor_measure -= 1;
                self.update_ab_repeat_follow_end_with_cursor();
                self.preview_current_target_if_stopped();
            }
            KeyCode::Char('H') => {
                self.start_history_overlay();
            }
            KeyCode::Char('l') | KeyCode::Right
                if self.editor.cursor_measure < self.editor.measures =>
            {
                self.editor.cursor_measure += 1;
                self.update_ab_repeat_follow_end_with_cursor();
                self.preview_current_target_if_stopped();
            }
            KeyCode::Char('j') | KeyCode::Down
                if self.editor.cursor_track + 1 < self.editor.tracks =>
            {
                self.editor.cursor_track += 1;
                self.preview_current_target_if_stopped();
            }
            KeyCode::Char('k') | KeyCode::Up if self.editor.cursor_track > 0 => {
                self.editor.cursor_track -= 1;
                self.preview_current_target_if_stopped();
            }
            KeyCode::Char('M') => {
                self.editor.cursor_track = self.editor.tracks / 2;
            }
            KeyCode::Char('L') => {
                self.editor.cursor_track = self.editor.tracks - 1;
            }

            KeyCode::Char('i') => self.open_mml_overlay_or_insert(),
            KeyCode::Char('m') => {
                self.overlays.mixer.cursor_track = self
                    .editor
                    .cursor_track
                    .clamp(FIRST_PLAYABLE_TRACK, self.editor.tracks - 1);
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

            KeyCode::Char('s') if self.editor.cursor_track >= FIRST_PLAYABLE_TRACK => {
                if !self.solo_mode_active() {
                    self.solo_tracks.fill(false);
                    self.solo_tracks[self.editor.cursor_track] = true;
                } else if let Some(is_solo) = self.solo_tracks.get_mut(self.editor.cursor_track) {
                    *is_solo = !*is_solo;
                }
                self.sync_playback_mml_state();
            }

            KeyCode::Char('g') => self.apply_generate_to_current_measure(),
            KeyCode::Char('r') => {
                if self.restore_default_tempo_init_if_empty() {
                    return DawNormalAction::Continue;
                }
                if let Err(message) = self.apply_random_patch_to_track(self.editor.cursor_track) {
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
