//! DawApp のプレビュー再生

use std::sync::atomic::Ordering;
use std::sync::Arc;

use clack_host::prelude::PluginEntry;
use cmrt_core::NativeRenderProbeContext;

use super::playback::try_get_cached_samples;
use super::{DawApp, DawPlayState, FIRST_PLAYABLE_TRACK, MAX_CACHED_SAMPLES};
use crate::history::daw_cache_mml_hash;

#[path = "preview/render.rs"]
mod render;

pub(super) use render::begin_preview_output;
use render::{
    insert_overlay_preview_cache, overlay_preview_cache_key, render_mixed_preview_tracks,
};

impl DawApp {
    pub(super) fn prefetch_preview_navigation_cache<F>(
        &self,
        current: usize,
        item_count: usize,
        page_size: usize,
        mut preview_for_index: F,
    ) where
        F: FnMut(usize) -> Option<(usize, Vec<String>)>,
    {
        let track_gains = self.playback_track_gains();
        for index in crate::ui_utils::predicted_navigation_indices(current, item_count, page_size) {
            if let Some((measure_index, track_mmls)) = preview_for_index(index) {
                self.prefetch_preview_snapshot(measure_index, track_mmls, track_gains.clone());
            }
        }
    }

    pub(super) fn prefetch_preview_snapshot(
        &self,
        measure_index: usize,
        track_mmls: Vec<String>,
        track_gains: Vec<f32>,
    ) {
        let active_tracks: Vec<usize> = (FIRST_PLAYABLE_TRACK..self.tracks)
            .filter(|&track| {
                track_gains.get(track).copied().unwrap_or(1.0) > 0.0
                    && track_mmls
                        .get(track)
                        .map(|mml| !mml.trim().is_empty())
                        .unwrap_or(false)
            })
            .collect();
        if active_tracks.is_empty() {
            return;
        }

        let cache_key = overlay_preview_cache_key(measure_index, &track_mmls, &track_gains);
        if self
            .overlay_preview_cache
            .lock()
            .unwrap()
            .contains_key(&cache_key)
        {
            return;
        }

        let measure_samples = self.measure_duration_samples();
        if measure_samples > MAX_CACHED_SAMPLES {
            return;
        }

        #[cfg(test)]
        if self.entry_ptr == 0 {
            insert_overlay_preview_cache(
                &mut self.overlay_preview_cache.lock().unwrap(),
                cache_key,
                Arc::new(Vec::new()),
            );
            return;
        }
        let cfg = Arc::clone(&self.cfg);
        let overlay_preview_cache = Arc::clone(&self.overlay_preview_cache);
        let entry_ptr = self.entry_ptr;
        let active_track_count = active_tracks.len();
        std::thread::spawn(move || {
            // SAFETY: `entry_ptr` は `main` から渡された `PluginEntry` を指し、
            // アプリ終了まで生存する契約で `DawApp` に保持されている。
            let entry_ref: &PluginEntry = unsafe { &*(entry_ptr as *const PluginEntry) };
            let daw_cfg = (*cfg).clone();
            let Some(samples) = render_mixed_preview_tracks(
                entry_ref,
                &daw_cfg,
                measure_samples,
                &active_tracks,
                &track_mmls,
                &track_gains,
                |track, mml| {
                    NativeRenderProbeContext::preview_prefetch(
                        track,
                        measure_index,
                        active_track_count,
                        daw_cache_mml_hash(mml),
                        daw_cfg.offline_render_workers,
                    )
                },
            ) else {
                return;
            };
            insert_overlay_preview_cache(
                &mut overlay_preview_cache.lock().unwrap(),
                cache_key,
                Arc::new(samples),
            );
        });
    }

    pub(super) fn start_preview_with_snapshot(
        &self,
        measure_index: usize,
        track_mmls: Vec<String>,
        track_gains: Vec<f32>,
    ) {
        let active_tracks: Vec<usize> = (FIRST_PLAYABLE_TRACK..self.tracks)
            .filter(|&track| {
                track_gains.get(track).copied().unwrap_or(1.0) > 0.0
                    && track_mmls
                        .get(track)
                        .map(|mml| !mml.trim().is_empty())
                        .unwrap_or(false)
            })
            .collect();
        if active_tracks.is_empty() {
            return;
        }

        let measure_samples = self.measure_duration_samples();
        let play_state = Arc::clone(&self.play_state);
        let play_transition_lock = Arc::clone(&self.play_transition_lock);
        let preview_session = Arc::clone(&self.preview_session);
        let preview_sink = Arc::clone(&self.preview_sink);
        let play_position = Arc::clone(&self.play_position);
        let cache = Arc::clone(&self.cache);
        let overlay_preview_cache = Arc::clone(&self.overlay_preview_cache);
        let cfg = Arc::clone(&self.cfg);
        let log_lines = Arc::clone(&self.log_lines);
        let entry_ptr = self.entry_ptr;
        let tracks = self.tracks;
        let overlay_cache_key = overlay_preview_cache_key(measure_index, &track_mmls, &track_gains);
        let active_track_count = active_tracks.len();

        let session = {
            let _transition_guard = play_transition_lock.lock().unwrap();
            if let Some(sink) = preview_sink.lock().unwrap().take() {
                sink.stop();
            }
            *play_position.lock().unwrap() = None;
            let session = preview_session.fetch_add(1, Ordering::AcqRel) + 1;
            *play_state.lock().unwrap() = DawPlayState::Preview;
            session
        };
        crate::logging::append_log_line(&log_lines, format!("preview: meas{}", measure_index + 1));

        std::thread::spawn(move || {
            // SAFETY: `entry_ptr` は `main` から渡された `PluginEntry` を指し、
            // アプリ終了まで生存する契約で `DawApp` に保持されている。
            let entry_ref: &PluginEntry = unsafe { &*(entry_ptr as *const PluginEntry) };
            let daw_cfg = (*cfg).clone();
            let sample_rate = daw_cfg.sample_rate as u32;

            let Ok((_stream, stream_handle)) = rodio::OutputStream::try_default() else {
                crate::logging::append_log_line(&log_lines, "preview: audio init failed");
                let mut state = play_state.lock().unwrap();
                if *state == DawPlayState::Preview
                    && preview_session.load(Ordering::Acquire) == session
                {
                    *state = DawPlayState::Idle;
                    drop(state);
                    *preview_sink.lock().unwrap() = None;
                    *play_position.lock().unwrap() = None;
                }
                return;
            };
            let Ok(sink) = rodio::Sink::try_new(&stream_handle) else {
                crate::logging::append_log_line(&log_lines, "preview: sink init failed");
                let mut state = play_state.lock().unwrap();
                if *state == DawPlayState::Preview
                    && preview_session.load(Ordering::Acquire) == session
                {
                    *state = DawPlayState::Idle;
                    drop(state);
                    *preview_sink.lock().unwrap() = None;
                    *play_position.lock().unwrap() = None;
                }
                return;
            };
            let shared_sink = Arc::new(sink);

            let samples_opt = if let Some(samples) = overlay_preview_cache
                .lock()
                .unwrap()
                .get(&overlay_cache_key)
                .cloned()
            {
                crate::logging::append_log_line(
                    &log_lines,
                    format!("meas{}: overlay cache hit", measure_index + 1),
                );
                Some((samples, true))
            } else if let Some(cached) = try_get_cached_samples(
                &cache,
                measure_index + 1,
                measure_samples,
                tracks,
                &track_gains,
            ) {
                if cached.cached_tracks.len() != active_tracks.len() {
                    crate::logging::append_log_line(
                        &log_lines,
                        format!("meas{}: render", measure_index + 1),
                    );
                    render_mixed_preview_tracks(
                        entry_ref,
                        &daw_cfg,
                        measure_samples,
                        &active_tracks,
                        &track_mmls,
                        &track_gains,
                        |track, mml| {
                            NativeRenderProbeContext::preview(
                                track,
                                measure_index,
                                active_track_count,
                                daw_cache_mml_hash(mml),
                                daw_cfg.offline_render_workers,
                            )
                        },
                    )
                    .map(|samples| (Arc::new(samples), false))
                } else {
                    crate::logging::append_log_line(
                        &log_lines,
                        format!(
                            "meas{}: cache hit {}",
                            measure_index + 1,
                            if cached.cached_tracks.is_empty() {
                                "empty-tracks".to_string()
                            } else {
                                cached
                                    .cached_tracks
                                    .iter()
                                    .map(|track| format!("track{track}/meas{}", measure_index + 1))
                                    .collect::<Vec<_>>()
                                    .join(", ")
                            }
                        ),
                    );
                    Some((Arc::new(cached.samples), false))
                }
            } else {
                crate::logging::append_log_line(
                    &log_lines,
                    format!("meas{}: render", measure_index + 1),
                );
                render_mixed_preview_tracks(
                    entry_ref,
                    &daw_cfg,
                    measure_samples,
                    &active_tracks,
                    &track_mmls,
                    &track_gains,
                    |track, mml| {
                        NativeRenderProbeContext::preview(
                            track,
                            measure_index,
                            active_track_count,
                            daw_cache_mml_hash(mml),
                            daw_cfg.offline_render_workers,
                        )
                    },
                )
                .map(|samples| (Arc::new(samples), false))
            };

            if let Some((samples, cache_hit)) = samples_opt {
                if !cache_hit {
                    insert_overlay_preview_cache(
                        &mut overlay_preview_cache.lock().unwrap(),
                        overlay_cache_key,
                        Arc::clone(&samples),
                    );
                }
                let preview_active = begin_preview_output(
                    &play_transition_lock,
                    &play_state,
                    &play_position,
                    &preview_session,
                    session,
                    measure_index,
                    || {
                        let source = rodio::buffer::SamplesBuffer::new(
                            2,
                            sample_rate,
                            samples.as_ref().clone(),
                        );
                        *preview_sink.lock().unwrap() = Some(Arc::clone(&shared_sink));
                        shared_sink.append(source);
                    },
                );
                if preview_active {
                    shared_sink.sleep_until_end();
                }
            } else {
                crate::logging::append_log_line(
                    &log_lines,
                    format!("meas{}: render error", measure_index + 1),
                );
            }

            let mut state = play_state.lock().unwrap();
            if *state == DawPlayState::Preview && preview_session.load(Ordering::Acquire) == session
            {
                *state = DawPlayState::Idle;
                drop(state);
                preview_sink.lock().unwrap().take();
                *play_position.lock().unwrap() = None;
                crate::logging::append_log_line(&log_lines, "preview: finished");
            }
        });
    }

    /// 指定された小節を一度だけ再生するプレビュー（ループなし）
    pub(super) fn start_preview(&self, measure_index: usize) {
        let measure_track_mmls = self.build_measure_track_mmls();
        let track_mmls = measure_track_mmls
            .get(measure_index)
            .cloned()
            .unwrap_or_else(|| vec![String::new(); self.tracks]);
        let track_gains = self.playback_track_gains();
        self.start_preview_with_snapshot(measure_index, track_mmls, track_gains);
    }

    pub(super) fn start_preview_on_tracks(&self, measure_index: usize, selected_tracks: &[usize]) {
        let mut track_mmls = vec![String::new(); self.tracks];
        let mut track_gains = vec![0.0; self.tracks];
        let displayed_measure = measure_index + 1;
        for &track in selected_tracks {
            if track < FIRST_PLAYABLE_TRACK || track >= self.tracks {
                continue;
            }
            let notes = self
                .data
                .get(track)
                .and_then(|row| row.get(displayed_measure))
                .map(String::as_str)
                .unwrap_or_default();
            if notes.trim().is_empty() {
                continue;
            }
            track_mmls[track] = self.build_cell_mml(track, displayed_measure);
            track_gains[track] = 10.0f32.powf(self.track_volume_db(track) as f32 / 20.0);
        }
        self.start_preview_with_snapshot(measure_index, track_mmls, track_gains);
    }
}

#[cfg(test)]
#[path = "../tests/daw/preview.rs"]
mod tests;
