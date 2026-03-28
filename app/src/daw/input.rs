//! DAW モードのキー入力処理

use crossterm::event::{KeyCode, KeyModifiers};
use mmlabc_to_smf::mml_preprocessor;
use serde_json::Value;

use super::{
    playback_util::effective_measure_count, AbRepeatState, DawApp, DawHistoryPane, DawMode,
    DawNormalAction, DawPlayState, FIRST_PLAYABLE_TRACK,
};

const PATCH_JSON_KEY: &str = "Surge XT patch";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum NormalPlaybackShortcut {
    PreviewCurrentTrack,
    PreviewAllTracks,
    PlayFromCursor,
}

fn normal_playback_shortcut(
    key_event: crossterm::event::KeyEvent,
) -> Option<NormalPlaybackShortcut> {
    let shift = key_event.modifiers.contains(KeyModifiers::SHIFT);
    match key_event.code {
        KeyCode::Enter | KeyCode::Char(' ') if shift => {
            Some(NormalPlaybackShortcut::PreviewAllTracks)
        }
        KeyCode::Enter | KeyCode::Char(' ') => Some(NormalPlaybackShortcut::PreviewCurrentTrack),
        KeyCode::Char('p') | KeyCode::Char('P') if shift => {
            Some(NormalPlaybackShortcut::PlayFromCursor)
        }
        _ => None,
    }
}

fn preview_target_tracks(
    tracks: usize,
    cursor_track: usize,
    preview_all_tracks: bool,
) -> Option<Vec<usize>> {
    if preview_all_tracks {
        return Some((FIRST_PLAYABLE_TRACK..tracks).collect());
    }
    if cursor_track < FIRST_PLAYABLE_TRACK || cursor_track >= tracks {
        return None;
    }
    Some(vec![cursor_track])
}

fn resolve_playback_start_measure_index(
    cursor_measure_index: Option<usize>,
    shortcut: NormalPlaybackShortcut,
) -> Option<usize> {
    match shortcut {
        NormalPlaybackShortcut::PlayFromCursor => cursor_measure_index,
        NormalPlaybackShortcut::PreviewCurrentTrack | NormalPlaybackShortcut::PreviewAllTracks => {
            Some(0)
        }
    }
}

fn format_random_patch_hot_reload_log(
    track: usize,
    displayed_measure_index: Option<usize>,
    old_effective_count: Option<usize>,
    new_effective_count: Option<usize>,
    old_measure_samples: usize,
    new_measure_samples: usize,
) -> String {
    let displayed = displayed_measure_index
        .map(|measure_index| format!("meas{}", measure_index + 1))
        .unwrap_or_else(|| "none".to_string());
    format!(
        "play: hot reload random patch track{track} display={displayed} effective_count={old_effective_count:?}->{new_effective_count:?} measure_samples={old_measure_samples}->{new_measure_samples}"
    )
}

impl DawApp {
    fn push_front_dedup(items: &mut Vec<String>, item: String) {
        if item.trim().is_empty() {
            return;
        }
        if let Some(index) = items.iter().position(|existing| existing == &item) {
            if index == 0 {
                return;
            }
            items.remove(index);
        }
        items.insert(0, item);
        if items.len() > 100 {
            items.truncate(100);
        }
    }

    fn extract_patch_phrase(mml: &str) -> Option<(String, String)> {
        let preprocessed = mml_preprocessor::extract_embedded_json(mml);
        let patch_name = preprocessed
            .embedded_json
            .as_deref()
            .and_then(|json| serde_json::from_str::<Value>(json).ok())
            .and_then(|value| {
                value
                    .get(PATCH_JSON_KEY)
                    .and_then(Value::as_str)
                    .map(str::to_string)
            })?;
        let phrase = preprocessed.remaining_mml.trim().to_string();
        Some((patch_name, phrase))
    }

    fn build_patch_json(patch_name: &str) -> String {
        serde_json::json!({ PATCH_JSON_KEY: patch_name }).to_string()
    }

    fn current_track_patch_name(&self) -> Option<String> {
        if self.cursor_track < FIRST_PLAYABLE_TRACK {
            return None;
        }
        Self::extract_patch_phrase(&self.data[self.cursor_track][0])
            .map(|(patch_name, _)| patch_name)
    }

    pub(super) fn history_overlay_history_items(&self) -> Vec<String> {
        if let Some(patch_name) = self.history_overlay_patch_name.as_deref() {
            self.patch_phrase_store
                .patches
                .get(patch_name)
                .map(|state| state.history.clone())
                .filter(|items| !items.is_empty())
                .unwrap_or_else(|| vec!["c".to_string()])
        } else {
            self.patch_phrase_store
                .notepad
                .history
                .iter()
                .filter(|item| Self::extract_patch_phrase(item).is_some())
                .cloned()
                .collect()
        }
    }

    pub(super) fn history_overlay_favorite_items(&self) -> Vec<String> {
        if let Some(patch_name) = self.history_overlay_patch_name.as_deref() {
            self.patch_phrase_store
                .patches
                .get(patch_name)
                .map(|state| state.favorites.clone())
                .filter(|items| !items.is_empty())
                .unwrap_or_else(|| vec!["c".to_string()])
        } else {
            self.patch_phrase_store
                .notepad
                .favorites
                .iter()
                .filter(|item| Self::extract_patch_phrase(item).is_some())
                .cloned()
                .collect()
        }
    }

    fn sync_history_overlay_cursors(&mut self) {
        let history_len = self.history_overlay_history_items().len();
        if history_len == 0 {
            self.history_overlay_history_cursor = 0;
        } else {
            self.history_overlay_history_cursor =
                self.history_overlay_history_cursor.min(history_len - 1);
        }

        let favorites_len = self.history_overlay_favorite_items().len();
        if favorites_len == 0 {
            self.history_overlay_favorites_cursor = 0;
        } else {
            self.history_overlay_favorites_cursor =
                self.history_overlay_favorites_cursor.min(favorites_len - 1);
        }
    }

    fn start_history_overlay(&mut self) {
        if self.cursor_track < FIRST_PLAYABLE_TRACK {
            return;
        }
        self.history_overlay_patch_name = self.current_track_patch_name();
        self.history_overlay_focus = DawHistoryPane::History;
        self.history_overlay_history_cursor = 0;
        self.history_overlay_favorites_cursor = 0;
        self.sync_history_overlay_cursors();
        self.mode = DawMode::History;
    }

    fn selected_history_overlay_item(&self) -> Option<String> {
        match self.history_overlay_focus {
            DawHistoryPane::History => self
                .history_overlay_history_items()
                .get(self.history_overlay_history_cursor)
                .cloned(),
            DawHistoryPane::Favorites => self
                .history_overlay_favorite_items()
                .get(self.history_overlay_favorites_cursor)
                .cloned(),
        }
    }

    fn history_overlay_target_measure(&self) -> usize {
        self.cursor_measure.max(1).min(self.measures)
    }

    fn apply_history_overlay_selection(&mut self, selected: String) {
        let target_measure = self.history_overlay_target_measure();
        if self.cursor_measure == 0 {
            self.cursor_measure = target_measure;
            self.update_ab_repeat_follow_end_with_cursor();
        }

        match self.history_overlay_patch_name.clone() {
            Some(patch_name) => {
                let previous = self.data[self.cursor_track][target_measure]
                    .trim()
                    .to_string();
                if !previous.is_empty() {
                    let state = self
                        .patch_phrase_store
                        .patches
                        .entry(patch_name)
                        .or_default();
                    Self::push_front_dedup(&mut state.history, previous);
                }

                if self.commit_insert_cell(self.cursor_track, target_measure, &selected) {
                    self.save();
                    self.sync_playback_mml_state();
                }
            }
            None => {
                let Some((patch_name, phrase)) = Self::extract_patch_phrase(&selected) else {
                    return;
                };
                let patch_json = Self::build_patch_json(&patch_name);
                let previous = self.data[self.cursor_track][target_measure]
                    .trim()
                    .to_string();
                if !previous.is_empty() {
                    Self::push_front_dedup(
                        &mut self.patch_phrase_store.notepad.history,
                        format!("{patch_json} {previous}"),
                    );
                }

                let init_changed = self.commit_insert_cell(self.cursor_track, 0, &patch_json);
                let phrase_changed =
                    self.commit_insert_cell(self.cursor_track, target_measure, &phrase);
                if init_changed || phrase_changed {
                    self.save();
                    self.sync_playback_mml_state();
                }
            }
        }

        let _ = crate::history::save_patch_phrase_store(&self.patch_phrase_store);
        if *self.play_state.lock().unwrap() == DawPlayState::Idle
            && target_measure > 0
            && self.entry_ptr != 0
        {
            self.start_preview(target_measure - 1);
        }
        self.mode = DawMode::Normal;
    }

    fn cursor_play_measure_index(&self) -> Option<usize> {
        // cursor_measure の 0 は Init 列なので対象外。
        // A-B リピートは通常 meas のみを扱うため、1-based の小節番号を 0-based index に変換する。
        self.cursor_measure.checked_sub(1)
    }

    fn update_ab_repeat_follow_end_with_cursor(&self) {
        let Some(end_measure_index) = self.cursor_play_measure_index() else {
            return;
        };
        let mut ab_repeat = self.ab_repeat.lock().unwrap();
        if let AbRepeatState::FixStart {
            start_measure_index,
            ..
        } = *ab_repeat
        {
            *ab_repeat = AbRepeatState::FixStart {
                start_measure_index,
                end_measure_index,
            };
        }
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

    fn sync_playback_mml_state(&self) {
        let new_mmls = self.build_measure_mmls();
        let new_track_mmls = self.build_measure_track_mmls();
        let new_samples = self.measure_duration_samples();
        let new_track_gains = self.playback_track_gains();
        *self.play_measure_mmls.lock().unwrap() = new_mmls;
        *self.play_measure_track_mmls.lock().unwrap() = new_track_mmls;
        *self.play_measure_samples.lock().unwrap() = new_samples;
        *self.play_track_gains.lock().unwrap() = new_track_gains;
    }

    fn start_preview_for_target_tracks(&self, preview_all_tracks: bool) {
        let play_state = *self.play_state.lock().unwrap();
        if play_state == DawPlayState::Playing {
            return;
        }
        if play_state == DawPlayState::Preview {
            self.stop_play();
        }
        let Some(measure_index) = self.cursor_play_measure_index() else {
            return;
        };
        let Some(target_tracks) =
            preview_target_tracks(self.tracks, self.cursor_track, preview_all_tracks)
        else {
            return;
        };
        self.start_preview_on_tracks(measure_index, &target_tracks);
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
        if self.entry_ptr != 0 {
            return false;
        }
        let measure_index = self.cursor_play_measure_index().unwrap_or(0);
        if *self.play_state.lock().unwrap() == DawPlayState::Preview {
            self.stop_play();
        }
        *self.play_state.lock().unwrap() = DawPlayState::Preview;
        *self.play_position.lock().unwrap() = Some(super::PlayPosition {
            measure_index,
            measure_start: std::time::Instant::now(),
        });
        self.append_log_line(format!("preview: meas{}", measure_index + 1));
        true
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

    // ─── INSERT モード ────────────────────────────────────────

    fn commit_insert_cell(&mut self, track: usize, measure: usize, text: &str) -> bool {
        if self.data[track][measure] == text {
            return false;
        }
        self.data[track][measure] = text.to_string();
        self.invalidate_cell(track, measure);
        self.kick_cache(track, measure);
        // track0 または音色セル変更時は依存セルも再キャッシュする
        self.invalidate_and_kick_dependent_cells(track, measure);
        true
    }

    pub(super) fn start_insert(&mut self) {
        let mut ta = tui_textarea::TextArea::default();
        for ch in self.data[self.cursor_track][self.cursor_measure].chars() {
            ta.insert_char(ch);
        }
        self.textarea = ta;
        self.mode = DawMode::Insert;
    }

    /// 編集内容を確定してキャッシュ更新・保存を行う
    pub(super) fn commit_insert(&mut self) {
        let text = self.textarea.lines().join("");
        let changed = self.commit_insert_cell(self.cursor_track, self.cursor_measure, &text);

        if !changed {
            return;
        }

        self.save();

        // hot reload: 次の再生ループから新しい MML と小節サンプル数を反映する
        // ロックを最小限に保つため、build_measure_mmls() と measure_duration_samples() を
        // ロック取得前に実行する
        self.sync_playback_mml_state();
    }

    // ─── キー処理 ─────────────────────────────────────────────

    pub(super) fn handle_help(&mut self, key: KeyCode) {
        if key == KeyCode::Esc {
            self.mode = DawMode::Normal;
        }
    }

    pub(super) fn handle_mixer(&mut self, key: KeyCode) {
        match key {
            KeyCode::Esc => {
                self.mode = DawMode::Normal;
            }
            KeyCode::Char('h') | KeyCode::Left => {
                if self.mixer_cursor_track > FIRST_PLAYABLE_TRACK {
                    self.mixer_cursor_track -= 1;
                }
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.mixer_cursor_track + 1 < self.tracks {
                    self.mixer_cursor_track += 1;
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.adjust_track_volume_db(self.mixer_cursor_track, -3) {
                    self.save();
                    self.sync_playback_mml_state();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.adjust_track_volume_db(self.mixer_cursor_track, 3) {
                    self.save();
                    self.sync_playback_mml_state();
                }
            }
            _ => {}
        }
    }

    pub(super) fn handle_history_overlay(&mut self, key: KeyCode) {
        let history_len = self.history_overlay_history_items().len();
        let favorites_len = self.history_overlay_favorite_items().len();

        match key {
            KeyCode::Esc => {
                self.mode = DawMode::Normal;
            }
            KeyCode::Char('h') | KeyCode::Left => {
                self.history_overlay_focus = DawHistoryPane::History;
                self.sync_history_overlay_cursors();
            }
            KeyCode::Char('l') | KeyCode::Right => {
                self.history_overlay_focus = DawHistoryPane::Favorites;
                self.sync_history_overlay_cursors();
            }
            KeyCode::Char('j') | KeyCode::Down => {
                match self.history_overlay_focus {
                    DawHistoryPane::History
                        if self.history_overlay_history_cursor + 1 < history_len =>
                    {
                        self.history_overlay_history_cursor += 1;
                    }
                    DawHistoryPane::Favorites
                        if self.history_overlay_favorites_cursor + 1 < favorites_len =>
                    {
                        self.history_overlay_favorites_cursor += 1;
                    }
                    _ => {}
                }
                self.sync_history_overlay_cursors();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                match self.history_overlay_focus {
                    DawHistoryPane::History if self.history_overlay_history_cursor > 0 => {
                        self.history_overlay_history_cursor -= 1;
                    }
                    DawHistoryPane::Favorites if self.history_overlay_favorites_cursor > 0 => {
                        self.history_overlay_favorites_cursor -= 1;
                    }
                    _ => {}
                }
                self.sync_history_overlay_cursors();
            }
            KeyCode::Enter => {
                if let Some(selected) = self.selected_history_overlay_item() {
                    self.apply_history_overlay_selection(selected);
                }
            }
            _ => {}
        }
    }

    pub(super) fn handle_normal_key_event(
        &mut self,
        key_event: crossterm::event::KeyEvent,
    ) -> DawNormalAction {
        match normal_playback_shortcut(key_event) {
            Some(NormalPlaybackShortcut::PreviewCurrentTrack) => {
                self.start_preview_for_target_tracks(false);
                return DawNormalAction::Continue;
            }
            Some(NormalPlaybackShortcut::PreviewAllTracks) => {
                self.start_preview_for_target_tracks(true);
                return DawNormalAction::Continue;
            }
            Some(NormalPlaybackShortcut::PlayFromCursor) => {
                if *self.play_state.lock().unwrap() == DawPlayState::Idle {
                    self.start_play_from_cursor_measure();
                    return DawNormalAction::Continue;
                }
            }
            None => {}
        }

        match key_event.code {
            KeyCode::Char('q') => return DawNormalAction::QuitApp,
            KeyCode::Char('d') | KeyCode::Esc => return DawNormalAction::ReturnToTui,

            KeyCode::Char('h') | KeyCode::Left => {
                if self.cursor_measure > 0 {
                    self.cursor_measure -= 1;
                    self.update_ab_repeat_follow_end_with_cursor();
                    self.preview_current_target_if_stopped();
                }
            }
            KeyCode::Char('H') => {
                self.start_history_overlay();
            }
            KeyCode::Char('l') | KeyCode::Right => {
                if self.cursor_measure < self.measures {
                    self.cursor_measure += 1;
                    self.update_ab_repeat_follow_end_with_cursor();
                    self.preview_current_target_if_stopped();
                }
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if self.cursor_track + 1 < self.tracks {
                    self.cursor_track += 1;
                    self.preview_current_target_if_stopped();
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if self.cursor_track > 0 {
                    self.cursor_track -= 1;
                    self.preview_current_target_if_stopped();
                }
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

            KeyCode::Char('K') | KeyCode::Char('?') => self.mode = DawMode::Help,

            KeyCode::Char('p') => {
                let state = *self.play_state.lock().unwrap();
                if state == DawPlayState::Playing || state == DawPlayState::Preview {
                    self.stop_play();
                } else {
                    self.start_play();
                }
            }

            KeyCode::Char('a') => self.cycle_ab_repeat(),

            KeyCode::Char('s') => {
                if self.cursor_track >= FIRST_PLAYABLE_TRACK {
                    if !self.solo_mode_active() {
                        self.solo_tracks.fill(false);
                        self.solo_tracks[self.cursor_track] = true;
                    } else if let Some(is_solo) = self.solo_tracks.get_mut(self.cursor_track) {
                        *is_solo = !*is_solo;
                    }
                    self.sync_playback_mml_state();
                }
            }

            KeyCode::Char('r') => {
                // measure 0 にランダム音色を設定
                if let Some(patch) = self.pick_random_patch_name() {
                    let affected_measures: Vec<usize> = (1..=self.measures)
                        .filter(|&measure| !self.data[self.cursor_track][measure].trim().is_empty())
                        .collect();
                    self.data[self.cursor_track][0] =
                        format!("{{\"Surge XT patch\": \"{}\"}}", patch);
                    self.invalidate_cell(self.cursor_track, 0);
                    self.invalidate_dependent_cells(self.cursor_track, 0);
                    // 依存セルはまとめてキュー投入せず、次の再生小節を優先して 1 件ずつ予約する。
                    self.start_track_rerender_batch(
                        self.cursor_track,
                        &affected_measures,
                        "random patch update",
                    );
                    self.save();

                    // hot reload: 次の再生ループから新しい音色を反映する
                    // ロックを最小限に保つため、build_measure_mmls() と measure_duration_samples() を
                    // ロック取得前に実行する
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
                        self.cursor_track,
                        displayed_measure_index,
                        old_effective_count,
                        new_effective_count,
                        old_samples,
                        new_samples,
                    ));
                    self.sync_playback_mml_state();
                }
            }

            _ => {}
        }
        DawNormalAction::Continue
    }

    #[cfg(test)]
    pub(super) fn handle_normal(&mut self, key: KeyCode) -> DawNormalAction {
        self.handle_normal_key_event(crossterm::event::KeyEvent::new(key, KeyModifiers::NONE))
    }

    pub(super) fn handle_insert(&mut self, key_event: crossterm::event::KeyEvent) {
        if key_event.modifiers.contains(KeyModifiers::CONTROL) {
            match key_event.code {
                KeyCode::Char('c') => {
                    self.textarea.copy();
                    crate::clipboard::set_text(self.textarea.yank_text().to_string());
                    return;
                }
                KeyCode::Char('x') => {
                    self.textarea.cut();
                    return;
                }
                KeyCode::Char('v') => {
                    self.textarea.paste();
                    return;
                }
                _ => {}
            }
        }
        match key_event.code {
            KeyCode::Esc => {
                let confirmed_measure = self.cursor_measure;
                self.commit_insert();
                // 非演奏中の場合、確定した小節をプレビュー再生する
                if *self.play_state.lock().unwrap() == DawPlayState::Idle && confirmed_measure > 0 {
                    self.start_preview(confirmed_measure - 1);
                }
                self.mode = DawMode::Normal;
            }
            KeyCode::Enter => {
                // 確定 → 次の小節へ → INSERT 継続
                let confirmed_measure = self.cursor_measure;
                self.commit_insert();
                // 非演奏中の場合、確定した小節をプレビュー再生する
                if *self.play_state.lock().unwrap() == DawPlayState::Idle && confirmed_measure > 0 {
                    self.start_preview(confirmed_measure - 1);
                }
                if self.cursor_measure < self.measures {
                    self.cursor_measure += 1;
                    self.update_ab_repeat_follow_end_with_cursor();
                }
                self.start_insert();
            }
            _ => {
                self.textarea.input(key_event);
            }
        }
    }
}

#[cfg(test)]
#[path = "../tests/daw/input.rs"]
mod tests;
