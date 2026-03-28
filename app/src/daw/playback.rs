//! DawApp の演奏メソッド

use std::{
    sync::Arc,
    time::{Duration, Instant},
};

use clack_host::prelude::PluginEntry;
use cmrt_core::{mml_render_for_cache, CoreConfig};

mod cache_mixer;
mod measure_math;
mod measure_mixer;

use super::playback_util::play_start_log_lines;
pub(super) use super::playback_util::{effective_measure_count, loop_measure_summary_label};
use super::{DawApp, DawPlayState, PlayPosition};
use cache_mixer::{build_playback_measure_samples, PlaybackMeasureRequest};
pub(super) use cache_mixer::{pad_playback_measure_samples, try_get_cached_samples};
pub(super) use measure_math::{current_play_measure_index, following_measure_index};
use measure_math::{
    format_playback_future_append_log, format_playback_measure_advance_log,
    format_playback_measure_resolution_log, future_chunk_append_deadline,
    resolved_measure_start_after_append,
};
use measure_mixer::{mix_measure_chunk, ActiveMeasureLayer, PlaybackMeasureAudio};

#[derive(Clone)]
struct QueuedMeasure {
    measure_index: usize,
    measure_start: std::time::Instant,
    measure_duration: std::time::Duration,
}

fn measure_duration(sample_count: usize, sample_rate: u32) -> std::time::Duration {
    // sample_count はステレオのインターリーブ済みサンプル総数（L/R の合計要素数）。
    // そのため実時間は frames (= sample_count / 2) / sample_rate となり、
    // sample_count / (sample_rate * 2) と等価になる。
    std::time::Duration::from_secs_f64(sample_count as f64 / (sample_rate as f64 * 2.0))
}

const FUTURE_CHUNK_APPEND_MARGIN: Duration = Duration::from_millis(50);

/// 指定時刻まで再生継続中なら待機し、deadline 到達で `true` を返す。
///
/// 再生中に state が `Playing` 以外へ変わった場合は早期に `false` を返す。
fn wait_until_or_stop(play_state: &Arc<std::sync::Mutex<DawPlayState>>, deadline: Instant) -> bool {
    loop {
        if *play_state.lock().unwrap() != DawPlayState::Playing {
            return false;
        }

        let now = Instant::now();
        if now >= deadline {
            return true;
        }

        std::thread::sleep((deadline - now).min(Duration::from_millis(10)));
    }
}

use std::sync::atomic::Ordering;

impl DawApp {
    // ─── 演奏 ─────────────────────────────────────────────────

    pub(super) fn start_play(&self) {
        self.start_play_from_measure(0);
    }

    pub(super) fn start_play_from_measure(&self, start_measure_index: usize) {
        let measure_mmls = self.build_measure_mmls();
        let measure_track_mmls = self.build_measure_track_mmls();
        let track_gains = self.playback_track_gains();
        if measure_mmls.iter().all(|m| m.trim().is_empty()) {
            return;
        }

        // play 状態を最新の値で更新してからスレッドに共有する
        *self.play_measure_mmls.lock().unwrap() = measure_mmls;
        *self.play_measure_track_mmls.lock().unwrap() = measure_track_mmls;
        *self.play_measure_samples.lock().unwrap() = self.measure_duration_samples();
        *self.play_track_gains.lock().unwrap() = track_gains;

        let play_state = Arc::clone(&self.play_state);
        let play_position = Arc::clone(&self.play_position);
        let ab_repeat = Arc::clone(&self.ab_repeat);
        let play_measure_mmls = Arc::clone(&self.play_measure_mmls);
        let play_measure_track_mmls = Arc::clone(&self.play_measure_track_mmls);
        let play_measure_samples = Arc::clone(&self.play_measure_samples);
        let play_track_gains = Arc::clone(&self.play_track_gains);
        let render_lock = Arc::clone(&self.render_lock);
        let cache = Arc::clone(&self.cache);
        let cfg = Arc::clone(&self.cfg);
        let log_lines = Arc::clone(&self.log_lines);
        let entry_ptr = self.entry_ptr;
        let tracks = self.tracks;

        *play_state.lock().unwrap() = DawPlayState::Playing;
        crate::logging::append_log_line(&log_lines, "play: start");
        for line in play_start_log_lines(
            &self.play_measure_mmls.lock().unwrap(),
            self.ab_repeat_state(),
        ) {
            crate::logging::append_log_line(&log_lines, line);
        }

        std::thread::spawn(move || {
            // SAFETY: entry は main() のスタックに生存している
            let entry_ref: &PluginEntry = unsafe { &*(entry_ptr as *const PluginEntry) };
            let daw_cfg = (*cfg).clone();
            let sample_rate = daw_cfg.sample_rate as u32;

            // OutputStream と Sink をスレッドに 1 つだけ作成し、小節をまたいで再利用する。
            // これにより小節ごとのオーディオ初期化オーバーヘッドとグリッチを防ぐ。
            let Ok((_stream, stream_handle)) = rodio::OutputStream::try_default() else {
                // Audio init failed: only reset to Idle if we are still the active Playing session.
                crate::logging::append_log_line(&log_lines, "play: audio init failed");
                let mut state = play_state.lock().unwrap();
                if *state == DawPlayState::Playing {
                    *state = DawPlayState::Idle;
                    drop(state);
                    *play_position.lock().unwrap() = None;
                }
                return;
            };
            let Ok(sink) = rodio::Sink::try_new(&stream_handle) else {
                crate::logging::append_log_line(&log_lines, "play: sink init failed");
                let mut state = play_state.lock().unwrap();
                if *state == DawPlayState::Playing {
                    *state = DawPlayState::Idle;
                    drop(state);
                    *play_position.lock().unwrap() = None;
                }
                return;
            };

            let mut measure_index = start_measure_index;
            let mut current_measure = None::<QueuedMeasure>;
            let mut active_layers = Vec::<ActiveMeasureLayer>::new();

            'outer: loop {
                if *play_state.lock().unwrap() != DawPlayState::Playing {
                    break;
                }

                if current_measure.is_none() {
                    // 初回の現在小節だけはすぐに解決して再生を開始する。
                    let mmls = play_measure_mmls.lock().unwrap().clone();
                    let measure_track_mmls = play_measure_track_mmls.lock().unwrap().clone();
                    let track_gains = play_track_gains.lock().unwrap().clone();
                    let measure_samples = *play_measure_samples.lock().unwrap();
                    let effective_count = match effective_measure_count(&mmls) {
                        Some(n) => n,
                        None => break 'outer,
                    };
                    let ab_repeat_range =
                        (*ab_repeat.lock().unwrap()).normalized_range(effective_count);
                    let current_measure_index =
                        current_play_measure_index(measure_index, effective_count, ab_repeat_range);
                    crate::logging::append_log_line(
                        &log_lines,
                        format_playback_measure_resolution_log(
                            measure_index,
                            current_measure_index,
                            effective_count,
                        ),
                    );
                    let track_mmls = &measure_track_mmls[current_measure_index];
                    let playback_audio = match build_playback_measure_samples(
                        &cache,
                        PlaybackMeasureRequest {
                            measure_index: current_measure_index,
                            track_mmls,
                            measure_samples,
                            tracks,
                            track_gains: &track_gains,
                        },
                        &log_lines,
                        |_track, mml| {
                            let _guard = render_lock.lock().unwrap();
                            let core_cfg = CoreConfig::from(&daw_cfg);
                            mml_render_for_cache(mml, &core_cfg, entry_ref)
                        },
                    ) {
                        Ok(playback_audio) => playback_audio,
                        Err(_) => {
                            crate::logging::append_log_line(
                                &log_lines,
                                format!("meas{}: render error", current_measure_index + 1),
                            );
                            break 'outer;
                        }
                    };
                    let measure_start = std::time::Instant::now();
                    let measure_duration = measure_duration(measure_samples, sample_rate);
                    let PlaybackMeasureAudio { samples, source } = playback_audio;
                    let chunk = mix_measure_chunk(&mut active_layers, samples, measure_samples);
                    sink.append(rodio::buffer::SamplesBuffer::new(2, sample_rate, chunk));
                    *play_position.lock().unwrap() = Some(PlayPosition {
                        measure_index: current_measure_index,
                        measure_start,
                    });
                    crate::logging::append_log_line(
                        &log_lines,
                        source.build_log_line(current_measure_index + 1),
                    );
                    current_measure = Some(QueuedMeasure {
                        measure_index: current_measure_index,
                        measure_start,
                        measure_duration,
                    });
                }

                let current = current_measure.expect(
                    "BUG: current_measure must be initialized before lookahead; this indicates a logic error in the playback loop initialization",
                );

                let expected_next_measure_start = current.measure_start + current.measure_duration;
                let append_deadline = future_chunk_append_deadline(
                    current.measure_start,
                    current.measure_duration,
                    FUTURE_CHUNK_APPEND_MARGIN,
                );
                if !wait_until_or_stop(&play_state, append_deadline) {
                    break 'outer;
                }

                // 次小節の解決と append は境界直前まで遅らせ、再 render の反映余地を広げる。
                // そのぶん render が間に合わないケースを観測しやすいよう、append 実績もログに残す。
                let mmls = play_measure_mmls.lock().unwrap().clone();
                let measure_track_mmls = play_measure_track_mmls.lock().unwrap().clone();
                let track_gains = play_track_gains.lock().unwrap().clone();
                let measure_samples = *play_measure_samples.lock().unwrap();
                let effective_count = match effective_measure_count(&mmls) {
                    Some(n) => n,
                    None => break 'outer,
                };
                let ab_repeat_range =
                    (*ab_repeat.lock().unwrap()).normalized_range(effective_count);
                let lookahead_measure_index = following_measure_index(
                    current.measure_index,
                    effective_count,
                    ab_repeat_range,
                );
                let next_track_mmls = &measure_track_mmls[lookahead_measure_index];
                let next_playback_audio = match build_playback_measure_samples(
                    &cache,
                    PlaybackMeasureRequest {
                        measure_index: lookahead_measure_index,
                        track_mmls: next_track_mmls,
                        measure_samples,
                        tracks,
                        track_gains: &track_gains,
                    },
                    &log_lines,
                    |_track, mml| {
                        let _guard = render_lock.lock().unwrap();
                        let core_cfg = CoreConfig::from(&daw_cfg);
                        mml_render_for_cache(mml, &core_cfg, entry_ref)
                    },
                ) {
                    Ok(playback_audio) => playback_audio,
                    Err(_) => {
                        crate::logging::append_log_line(
                            &log_lines,
                            format!("meas{}: render error", lookahead_measure_index + 1),
                        );
                        break 'outer;
                    }
                };

                let next_measure_duration = measure_duration(measure_samples, sample_rate);
                let PlaybackMeasureAudio {
                    samples: next_samples,
                    source: next_source,
                } = next_playback_audio;
                let next_chunk =
                    mix_measure_chunk(&mut active_layers, next_samples, measure_samples);
                sink.append(rodio::buffer::SamplesBuffer::new(
                    2,
                    sample_rate,
                    next_chunk,
                ));
                let append_time = Instant::now();
                let next_measure_start =
                    resolved_measure_start_after_append(expected_next_measure_start, append_time);
                crate::logging::append_log_line(
                    &log_lines,
                    format_playback_future_append_log(
                        lookahead_measure_index,
                        append_time,
                        expected_next_measure_start,
                        FUTURE_CHUNK_APPEND_MARGIN,
                    ),
                );

                // 小節境界は期待再生開始時刻を基準にポーリングする。
                // 50ms 前を目標に次小節チャンクを append したうえで、
                // rodio::Sink には「現在キュー先頭の小節だけの終了」を待つ API がないため、
                // play_position / ログ更新だけを時間ベースで境界に同期する。
                if !wait_until_or_stop(&play_state, next_measure_start) {
                    break 'outer;
                }

                *play_position.lock().unwrap() = Some(PlayPosition {
                    measure_index: lookahead_measure_index,
                    measure_start: next_measure_start,
                });
                crate::logging::append_log_line(
                    &log_lines,
                    format_playback_measure_advance_log(
                        current.measure_index,
                        lookahead_measure_index,
                        effective_count,
                    ),
                );
                crate::logging::append_log_line(
                    &log_lines,
                    next_source.build_log_line(lookahead_measure_index + 1),
                );
                current_measure = Some(QueuedMeasure {
                    measure_index: lookahead_measure_index,
                    measure_start: next_measure_start,
                    measure_duration: next_measure_duration,
                });
                measure_index = lookahead_measure_index;
            }

            // Only reset to Idle if we are still the active Playing session.
            // An unconditional write would clobber a newer session started after stop.
            let mut state = play_state.lock().unwrap();
            if *state == DawPlayState::Playing {
                *state = DawPlayState::Idle;
                drop(state);
                *play_position.lock().unwrap() = None;
                crate::logging::append_log_line(&log_lines, "play: finished");
            }
        });
    }

    pub(super) fn stop_play(&self) {
        let _transition_guard = self.play_transition_lock.lock().unwrap();
        let prev_state = {
            let mut play_state = self.play_state.lock().unwrap();
            let prev_state = *play_state;
            *play_state = DawPlayState::Idle;
            prev_state
        };
        match prev_state {
            DawPlayState::Idle => {}
            DawPlayState::Preview => {
                self.preview_session.fetch_add(1, Ordering::AcqRel);
                if let Some(sink) = self.preview_sink.lock().unwrap().take() {
                    sink.stop();
                }
                self.append_log_line("preview: stop");
            }
            DawPlayState::Playing => self.append_log_line("play: stop"),
        }
        *self.play_position.lock().unwrap() = None;
    }
}

#[cfg(test)]
#[path = "../tests/daw/playback.rs"]
mod tests;
