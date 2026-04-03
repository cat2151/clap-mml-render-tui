//! DAW モードのキー入力処理

use mmlabc_to_smf::mml_preprocessor;
use serde_json::Value;
#[cfg(test)]
use std::time::Instant;

mod history;
mod insert;
mod mixer;
mod normal;
mod patch_select;

use super::{AbRepeatState, DawApp, FIRST_PLAYABLE_TRACK};

#[cfg(test)]
use normal::{
    normal_playback_shortcut, preview_target_tracks, resolve_playback_start_measure_index,
    NormalPlaybackShortcut,
};

const PATCH_JSON_KEY: &str = "Surge XT patch";
const PATCH_FILTER_QUERY_JSON_KEY: &str = "Surge XT patch filter";

impl DawApp {
    pub(in crate::daw) fn enter_help(&mut self) {
        self.help_origin = self.mode;
        self.mode = super::DawMode::Help;
    }

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

    fn extract_patch_json_and_phrase(mml: &str) -> Option<(Value, String)> {
        let preprocessed = mml_preprocessor::extract_embedded_json(mml);
        let value = preprocessed
            .embedded_json
            .as_deref()
            .and_then(|json| serde_json::from_str::<Value>(json).ok())?;
        Some((value, preprocessed.remaining_mml.trim().to_string()))
    }

    fn extract_patch_phrase(mml: &str) -> Option<(String, String)> {
        let (value, phrase) = Self::extract_patch_json_and_phrase(mml)?;
        let patch_name = value
            .get(PATCH_JSON_KEY)
            .and_then(Value::as_str)
            .map(str::to_string)?;
        Some((patch_name, phrase))
    }

    fn build_patch_json(patch_name: &str) -> String {
        Self::build_patch_json_with_filter_query(patch_name, None)
    }

    fn build_patch_json_with_filter_query(patch_name: &str, filter_query: Option<&str>) -> String {
        let mut patch_json = serde_json::json!({ PATCH_JSON_KEY: patch_name });
        if let Some(filter_query) = filter_query
            .map(str::trim)
            .filter(|query| !query.is_empty())
        {
            patch_json[PATCH_FILTER_QUERY_JSON_KEY] = Value::String(filter_query.to_string());
        }
        patch_json.to_string()
    }

    fn build_random_patch_json_with_filter_query(
        patch_name: &str,
        filter_query: Option<&str>,
    ) -> String {
        let patch_name = Value::String(patch_name.to_string());
        match filter_query
            .map(str::trim)
            .filter(|query| !query.is_empty())
        {
            Some(filter_query) => {
                let filter_query = Value::String(filter_query.to_string());
                format!(
                    r#"{{"{PATCH_JSON_KEY}": {patch_name}, "{PATCH_FILTER_QUERY_JSON_KEY}": {filter_query}}}"#
                )
            }
            None => format!(r#"{{"{PATCH_JSON_KEY}": {patch_name}}}"#),
        }
    }

    fn current_track_patch_name(&self) -> Option<String> {
        if self.cursor_track < FIRST_PLAYABLE_TRACK {
            return None;
        }
        Self::extract_patch_phrase(&self.data[self.cursor_track][0])
            .map(|(patch_name, _)| patch_name)
    }

    fn current_track_patch_filter_query(&self) -> Option<String> {
        if self.cursor_track < FIRST_PLAYABLE_TRACK {
            return None;
        }
        Self::extract_patch_json_and_phrase(&self.data[self.cursor_track][0]).and_then(
            |(value, _)| {
                value
                    .get(PATCH_FILTER_QUERY_JSON_KEY)
                    .and_then(Value::as_str)
                    .map(str::to_string)
            },
        )
    }

    fn mark_patch_phrase_store_dirty(&mut self) {
        self.patch_phrase_store_dirty = true;
    }

    pub(in crate::daw) fn flush_patch_phrase_store_if_dirty(&mut self) {
        if !self.patch_phrase_store_dirty {
            return;
        }
        let _ = crate::history::save_patch_phrase_store(&self.patch_phrase_store);
        self.patch_phrase_store_dirty = false;
    }

    fn record_current_measure_to_patch_history(&mut self, mml: &str) {
        let mml = mml.trim();
        if mml.is_empty() {
            return;
        }

        if self.cursor_measure > 0 {
            if let Some(patch_name) = self.current_track_patch_name() {
                let state = self
                    .patch_phrase_store
                    .patches
                    .entry(patch_name)
                    .or_default();
                Self::push_front_dedup(&mut state.history, mml.to_string());
                self.mark_patch_phrase_store_dirty();
                return;
            }
        }

        let Some((patch_name, phrase)) = Self::extract_patch_phrase(mml) else {
            return;
        };
        if phrase.is_empty() {
            return;
        }
        let state = self
            .patch_phrase_store
            .patches
            .entry(patch_name)
            .or_default();
        Self::push_front_dedup(&mut state.history, phrase);
        self.mark_patch_phrase_store_dirty();
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

    #[cfg(test)]
    fn try_start_preview_with_track_mmls_for_test(
        &mut self,
        measure_index: usize,
        track_mmls: Option<Vec<String>>,
    ) -> bool {
        if self.entry_ptr != 0 {
            return false;
        }
        if *self.play_state.lock().unwrap() == super::DawPlayState::Preview {
            self.stop_play();
        }
        if let Some(track_mmls) = track_mmls {
            if let Some(measure_track_mmls) = self
                .play_measure_track_mmls
                .lock()
                .unwrap()
                .get_mut(measure_index)
            {
                *measure_track_mmls = track_mmls;
            }
        }
        *self.play_state.lock().unwrap() = super::DawPlayState::Preview;
        *self.play_position.lock().unwrap() = Some(super::PlayPosition {
            measure_index,
            measure_start: Instant::now(),
        });
        self.append_log_line(format!("preview: meas{}", measure_index + 1));
        true
    }

    #[cfg(not(test))]
    fn try_start_preview_with_track_mmls_for_test(
        &mut self,
        _measure_index: usize,
        _track_mmls: Option<Vec<String>>,
    ) -> bool {
        false
    }
}

#[cfg(test)]
#[path = "../tests/daw/input.rs"]
mod tests;
