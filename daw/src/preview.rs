//! DawApp のプレビュー再生

use std::sync::Arc;

use self::cached_samples::try_get_cached_samples;
use self::service::OfflinePreviewRequest;
use super::{DawApp, FIRST_PLAYABLE_TRACK};
use cmrt_runtime::RealtimeAudioBackend;

mod cached_samples;
pub(crate) mod output;
pub(crate) mod overlay_cache;
mod play_server;
mod prefetch;
pub(crate) mod render;
pub(crate) mod service;

use render::{PreviewRenderProgress, PreviewRenderProgressPhase};

fn preview_render_progress_log_line(
    measure_index: usize,
    progress: PreviewRenderProgress,
) -> String {
    let track = crate::tracks::track_display_number(progress.track);
    let phase = match progress.phase {
        PreviewRenderProgressPhase::Started => "start".to_string(),
        PreviewRenderProgressPhase::Done { elapsed_ms } => format!("done ms={elapsed_ms}"),
        PreviewRenderProgressPhase::Error { elapsed_ms } => format!("error ms={elapsed_ms}"),
    };
    format!(
        "preview: render progress meas{} {}/{} track{} {phase}",
        measure_index + 1,
        progress.completed,
        progress.total,
        track,
    )
}

impl DawApp {
    pub(super) fn start_preview_with_snapshot(
        &self,
        measure_index: usize,
        track_mmls: Vec<String>,
        track_gains: Vec<f32>,
    ) {
        self.start_preview_with_snapshot_options(
            measure_index,
            track_mmls,
            track_gains,
            self.measure_duration_samples(),
            true,
        );
    }

    pub(super) fn start_uncached_preview_with_snapshot(
        &self,
        measure_index: usize,
        track_mmls: Vec<String>,
        track_gains: Vec<f32>,
        measure_samples: usize,
    ) {
        self.start_preview_with_snapshot_options(
            measure_index,
            track_mmls,
            track_gains,
            measure_samples,
            false,
        );
    }

    fn start_preview_with_snapshot_options(
        &self,
        measure_index: usize,
        track_mmls: Vec<String>,
        track_gains: Vec<f32>,
        measure_samples: usize,
        allow_cell_cache: bool,
    ) {
        let _slow = crate::performance_log::SlowOperation::with_context(
            "preview-start",
            format!(
                "measure={} track_count={} allow_cell_cache={allow_cell_cache}",
                measure_index + 1,
                track_mmls.len().max(track_gains.len())
            ),
        );
        self.stop_mml_overlay_sender();
        let active_tracks = active_preview_tracks(&track_mmls, &track_gains);
        if active_tracks.is_empty() {
            return;
        }

        if self.cfg.realtime_audio_backend == RealtimeAudioBackend::PlayServer {
            self.start_preview_with_snapshot_via_play_server(
                measure_index,
                track_mmls,
                active_tracks,
                measure_samples,
            );
            return;
        }

        self.start_offline_preview(
            OfflinePreviewRequest::current(
                measure_index,
                measure_samples,
                active_tracks,
                track_mmls,
                track_gains,
            ),
            allow_cell_cache,
        );
    }

    /// overlay preview cache に `request` の音があるときだけ、それを鳴らす。backend に関係なく
    /// rodio で鳴らす（cache の音は offline render が作ったもの）。鳴らしたら `true`。
    /// MML overlay sender（LIVE）の音は止めない。止めるかどうかは呼び出し側が決める。
    pub(super) fn start_overlay_cached_preview(&self, request: &OfflinePreviewRequest) -> bool {
        if request.active_tracks.is_empty()
            || self.render.preview_service().cached(request).is_none()
        {
            return false;
        }
        self.start_offline_preview(request.clone(), false);
        true
    }

    /// 別スレッドで overlay preview cache（`allow_cell_cache` なら cell cache も）を引き、
    /// 無ければ offline render を待って rodio で鳴らす。
    fn start_offline_preview(
        &self,
        preview_request: OfflinePreviewRequest,
        allow_cell_cache: bool,
    ) {
        let measure_index = preview_request.measure_index;
        let measure_samples = preview_request.measure_samples;
        let tracks = preview_request
            .track_mmls
            .len()
            .max(preview_request.track_gains.len());
        let preview_output = self.playback.preview_output.clone();
        let cache = Arc::clone(&self.cache);
        let preview_service = self.render.preview_service();
        let sample_rate = self.cfg.sample_rate as u32;
        let log_lines = Arc::clone(&self.log_lines);

        let session = preview_output.start_session();
        crate::append_log_line(&log_lines, format!("preview: meas{}", measure_index + 1));

        std::thread::spawn(move || {
            let Some(rodio_sample_rate) = rodio::SampleRate::new(sample_rate) else {
                crate::append_log_line(&log_lines, "preview: sample rate is zero");
                preview_output.finish_session(session);
                return;
            };
            // device sink を drop すると再生が止まるため、スレッドが終わるまで保持する。
            let Ok(device_sink) = cmrt_tui_core::audio_output::open_default_sink() else {
                crate::append_log_line(&log_lines, "preview: audio init failed");
                preview_output.finish_session(session);
                return;
            };
            let shared_sink = Arc::new(rodio::Player::connect_new(device_sink.mixer()));

            let render_preview = || {
                crate::append_log_line(&log_lines, format!("meas{}: render", measure_index + 1));
                preview_service
                    .render_blocking(&preview_request, |progress| {
                        crate::append_log_line(
                            &log_lines,
                            preview_render_progress_log_line(measure_index, progress),
                        );
                    })
                    .map(|render| (Arc::new(render.samples), false))
            };
            let cached_samples = preview_service.cached(&preview_request);
            let samples_opt = if let Some(samples) = cached_samples {
                crate::append_log_line(
                    &log_lines,
                    format!("meas{}: overlay cache hit", measure_index + 1),
                );
                Some((samples, true))
            } else if allow_cell_cache {
                if let Some(cached) = try_get_cached_samples(
                    &cache,
                    measure_index + 1,
                    measure_samples,
                    tracks,
                    &preview_request.track_gains,
                ) {
                    if cached.cached_tracks.len() != preview_request.active_tracks.len() {
                        render_preview()
                    } else {
                        crate::append_log_line(
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
                                        .map(|track| {
                                            let track = crate::tracks::track_display_number(*track);
                                            format!("track{track}/meas{}", measure_index + 1)
                                        })
                                        .collect::<Vec<_>>()
                                        .join(", ")
                                }
                            ),
                        );
                        Some((Arc::new(cached.samples), false))
                    }
                } else {
                    render_preview()
                }
            } else {
                render_preview()
            };

            if let Some((samples, cache_hit)) = samples_opt {
                if !cache_hit {
                    preview_service.store(&preview_request, Arc::clone(&samples));
                }
                let measure_duration = std::time::Duration::from_secs_f64(
                    measure_samples as f64 / (sample_rate as f64 * 2.0),
                );
                let preview_active = preview_output.enqueue_if_current(
                    session,
                    measure_index,
                    measure_duration,
                    Some(Arc::clone(&shared_sink)),
                    || {
                        let source = rodio::buffer::SamplesBuffer::new(
                            cmrt_tui_core::playback_session::STEREO,
                            rodio_sample_rate,
                            samples.as_ref().clone(),
                        );
                        shared_sink.append(source);
                    },
                );
                if preview_active {
                    shared_sink.sleep_until_end();
                }
            } else {
                crate::append_log_line(
                    &log_lines,
                    format!("meas{}: render error", measure_index + 1),
                );
            }

            if preview_output.finish_session(session) {
                crate::append_log_line(&log_lines, "preview: finished");
            }
        });
    }

    /// 指定された小節を一度だけ再生するプレビュー（ループなし）
    pub(super) fn start_preview(&self, measure_index: usize) {
        let measure_track_mmls = self.build_measure_track_mmls();
        let track_mmls = measure_track_mmls
            .get(measure_index)
            .cloned()
            .unwrap_or_else(|| vec![String::new(); self.editor.tracks]);
        let track_gains = self.playback_track_gains();
        self.start_preview_with_snapshot(measure_index, track_mmls, track_gains);
    }

    pub(super) fn start_preview_on_tracks(&self, measure_index: usize, selected_tracks: &[usize]) {
        let mut track_mmls = vec![String::new(); self.editor.tracks];
        let mut track_gains = vec![0.0; self.editor.tracks];
        let displayed_measure = measure_index + 1;
        for &track in selected_tracks {
            if track < FIRST_PLAYABLE_TRACK || track >= self.editor.tracks {
                continue;
            }
            if !crate::mml::cell_has_content(&self.editor.data, track, displayed_measure) {
                continue;
            }
            track_mmls[track] = self.build_cell_mml(track, displayed_measure);
            track_gains[track] = 10.0f32.powf(self.track_volume_db(track) as f32 / 20.0);
        }
        self.start_preview_with_snapshot(measure_index, track_mmls, track_gains);
    }
}

/// preview で鳴らす track。音量が 0 でなく、MML が空でないもの。
pub(crate) fn active_preview_tracks(track_mmls: &[String], track_gains: &[f32]) -> Vec<usize> {
    let tracks = track_mmls.len().max(track_gains.len());
    (FIRST_PLAYABLE_TRACK..tracks)
        .filter(|&track| {
            track_gains.get(track).copied().unwrap_or(1.0) > 0.0
                && track_mmls
                    .get(track)
                    .map(|mml| !mml.trim().is_empty())
                    .unwrap_or(false)
        })
        .collect()
}

#[cfg(test)]
mod tests;
