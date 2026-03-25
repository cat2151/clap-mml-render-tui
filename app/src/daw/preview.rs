//! DawApp のプレビュー再生

use std::sync::{Arc, Mutex};

use clack_host::prelude::PluginEntry;
use cmrt_core::{mml_render_for_cache, CoreConfig};

use super::playback::try_get_cached_samples;
use super::{DawApp, DawPlayState, PlayPosition};

fn begin_preview_output<F>(
    play_transition_lock: &Arc<Mutex<()>>,
    play_state: &Arc<Mutex<DawPlayState>>,
    play_position: &Arc<Mutex<Option<PlayPosition>>>,
    measure_index: usize,
    enqueue_audio: F,
) -> bool
where
    F: FnOnce(),
{
    let _transition_guard = play_transition_lock.lock().unwrap();
    if *play_state.lock().unwrap() != DawPlayState::Preview {
        return false;
    }
    *play_position.lock().unwrap() = Some(PlayPosition {
        measure_index,
        measure_start: std::time::Instant::now(),
    });
    enqueue_audio();
    true
}

impl DawApp {
    /// 指定された小節を一度だけ再生するプレビュー（ループなし）
    pub(super) fn start_preview(&self, measure_index: usize) {
        let mmls = self.build_measure_mmls();
        let mml = mmls.get(measure_index).cloned().unwrap_or_default();
        if mml.trim().is_empty() {
            return;
        }

        let measure_samples = self.measure_duration_samples();
        let play_state = Arc::clone(&self.play_state);
        let play_transition_lock = Arc::clone(&self.play_transition_lock);
        let play_position = Arc::clone(&self.play_position);
        let render_lock = Arc::clone(&self.render_lock);
        let cache = Arc::clone(&self.cache);
        let cfg = Arc::clone(&self.cfg);
        let log_lines = Arc::clone(&self.log_lines);
        let entry_ptr = self.entry_ptr;
        let tracks = self.tracks;

        *play_state.lock().unwrap() = DawPlayState::Preview;
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
                if *state == DawPlayState::Preview {
                    *state = DawPlayState::Idle;
                    drop(state);
                    *play_position.lock().unwrap() = None;
                }
                return;
            };
            let Ok(sink) = rodio::Sink::try_new(&stream_handle) else {
                crate::logging::append_log_line(&log_lines, "preview: sink init failed");
                let mut state = play_state.lock().unwrap();
                if *state == DawPlayState::Preview {
                    *state = DawPlayState::Idle;
                    drop(state);
                    *play_position.lock().unwrap() = None;
                }
                return;
            };

            let samples_opt = if let Some(cached) =
                try_get_cached_samples(&cache, measure_index + 1, measure_samples, tracks)
            {
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
                Some(cached.samples)
            } else {
                crate::logging::append_log_line(
                    &log_lines,
                    format!("meas{}: render", measure_index + 1),
                );
                let result = {
                    let _guard = render_lock.lock().unwrap();
                    let core_cfg = CoreConfig::from(&daw_cfg);
                    mml_render_for_cache(&mml, &core_cfg, entry_ref)
                };
                result.ok().map(|mut s| {
                    if s.len() < measure_samples {
                        s.resize(measure_samples, 0.0);
                    } else {
                        s.truncate(measure_samples);
                    }
                    s
                })
            };

            if let Some(samples) = samples_opt {
                let preview_active = begin_preview_output(
                    &play_transition_lock,
                    &play_state,
                    &play_position,
                    measure_index,
                    || {
                        let source = rodio::buffer::SamplesBuffer::new(2, sample_rate, samples);
                        sink.append(source);
                    },
                );
                if preview_active {
                    sink.sleep_until_end();
                }
            } else {
                crate::logging::append_log_line(
                    &log_lines,
                    format!("meas{}: render error", measure_index + 1),
                );
            }

            let mut state = play_state.lock().unwrap();
            if *state == DawPlayState::Preview {
                *state = DawPlayState::Idle;
                drop(state);
                *play_position.lock().unwrap() = None;
                crate::logging::append_log_line(&log_lines, "preview: finished");
            }
        });
    }
}

#[cfg(test)]
#[path = "../tests/daw/preview.rs"]
mod tests;
