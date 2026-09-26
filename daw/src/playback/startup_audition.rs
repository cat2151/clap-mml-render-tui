//! 起動直後の自動再生で、play server が鳴らせるようになるまでの間を埋める試聴ループ。
//!
//! 開始小節の cell cache を自プロセスで mix し、rodio でループ再生する（server を経由しない）。
//! 本演奏（[`super::live_cache`]）へ切り替えるのは「server が起動済み」かつ「その小節の
//! render が済んでいる」の両方が揃ってから。どちらかが欠けたまま本演奏を始めると、
//! playhead だけが進む無音の演奏になる。
//!
//! 切り替えてもループはすぐには止めない。本演奏は 1 小節目の WAV を server へ載せ終えるまで
//! 音が出ないので、止めるのは server 側の 1 小節目が鳴る時刻（`PlayPosition::measure_start`）。

use std::{
    collections::VecDeque,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};

use cmrt_runtime::RealtimeAudioBackend;
use rodio::Source;

use super::{current_play_measure_index, effective_measure_count};
use crate::{
    preview::cached_samples::try_get_cached_samples, CacheState, CellCache, DawApp, DawPlayState,
    PlayPosition, FIRST_PLAYABLE_TRACK,
};

/// 試聴ループ 1 回ぶんの状態。`DawPlaybackRuntime::startup_audition` に 1 つだけ置く。
pub(crate) struct StartupAudition {
    measure_index: usize,
    /// server の起動待ちが終わった（成功・失敗を問わない。失敗は本演奏側がログに出す）。
    server_settled: Arc<AtomicBool>,
    looper: Option<Arc<LooperControl>>,
    handed_off: bool,
}

impl StartupAudition {
    pub(crate) fn new(measure_index: usize, server_settled: Arc<AtomicBool>) -> Self {
        Self {
            measure_index,
            server_settled,
            looper: None,
            handed_off: false,
        }
    }
}

impl Drop for StartupAudition {
    fn drop(&mut self) {
        // DawApp ごと捨てられたときに、ループのスレッドを鳴らしっぱなしにしない。
        if let Some(looper) = &self.looper {
            looper.stop.store(true, Ordering::Release);
        }
    }
}

#[derive(Default)]
struct LooperControl {
    stop: AtomicBool,
    handed_off: AtomicBool,
    finished: AtomicBool,
}

impl LooperControl {
    fn should_stop(
        &self,
        play_state: &Mutex<DawPlayState>,
        position: &Mutex<Option<PlayPosition>>,
    ) -> bool {
        if self.stop.load(Ordering::Acquire) {
            return true;
        }
        if !self.handed_off.load(Ordering::Acquire) {
            return false;
        }
        if *play_state.lock().unwrap() != DawPlayState::Playing {
            return true;
        }
        position
            .lock()
            .unwrap()
            .as_ref()
            .is_some_and(|position| Instant::now() >= position.measure_start)
    }
}

/// 余韻（1 小節を超えたぶん）を先頭へ折り返し、ちょうど 1 小節の長さにする。
///
/// 切り捨てると、ループの継ぎ目で前の周回の余韻がぷつりと消える。
pub(crate) fn fold_into_loop(samples: &[f32], loop_len: usize) -> Vec<f32> {
    if loop_len == 0 {
        return samples.to_vec();
    }
    let mut looped = vec![0.0; loop_len];
    for (index, sample) in samples.iter().enumerate() {
        looped[index % loop_len] += sample;
    }
    looped
}

/// 小節 `measure`（1 始まり）で、聞こえる track の render が 1 本も待ち状態に残っていないか。
///
/// `Error` も「済んだ」に数える。待ち続けると本演奏が永久に始まらない。
pub(crate) fn measure_render_settled(
    cache: &[Vec<CellCache>],
    measure: usize,
    track_gains: &[f32],
) -> bool {
    (FIRST_PLAYABLE_TRACK..cache.len())
        .filter(|&track| track_gains.get(track).copied().unwrap_or(1.0) != 0.0)
        .all(|track| {
            !matches!(
                cache[track][measure].state,
                CacheState::Pending | CacheState::Rendering
            )
        })
}

impl DawApp {
    /// 起動直後の自動再生。カーソルの小節から始める（Shift+Space 相当）。
    ///
    /// 試聴ループが使えるのは `CachePlayer` backend だけ（鳴らす素材が cell cache なので）。
    pub(crate) fn start_autoplay_on_entry(&self) {
        let Some(play_server) = self
            .playback
            .realtime_play_server
            .as_ref()
            .filter(|_| self.cfg.realtime_audio_backend == RealtimeAudioBackend::CachePlayer)
            .cloned()
        else {
            self.start_play();
            return;
        };
        let measure_mmls = self.build_measure_mmls();
        let Some(effective_count) = effective_measure_count(&measure_mmls) else {
            return;
        };
        let ab_range = self.ab_repeat_state().normalized_range(effective_count);
        let measure_index = current_play_measure_index(
            self.cursor_play_measure_index().unwrap_or(0),
            effective_count,
            ab_range,
        );

        let server_settled = Arc::new(AtomicBool::new(false));
        let settled = Arc::clone(&server_settled);
        let log_lines = Arc::clone(&self.log_lines);
        std::thread::spawn(move || {
            if let Err(error) = play_server.ensure_started_for_fast_midi() {
                crate::append_log_line(
                    &log_lines,
                    format!("startup-audition: server start failed error=\"{error:#}\""),
                );
            }
            settled.store(true, Ordering::Release);
        });

        self.append_log_line(format!("startup-audition: begin meas{}", measure_index + 1));
        *self.playback.startup_audition.lock().unwrap() =
            Some(StartupAudition::new(measure_index, server_settled));
    }

    /// メインループが毎 tick 呼ぶ。ループの開始と、本演奏への切り替えを進める。
    pub(crate) fn pump_startup_audition(&self) {
        let mut slot = self.playback.startup_audition.lock().unwrap();
        let Some(audition) = slot.as_mut() else {
            return;
        };
        if audition.handed_off {
            if audition
                .looper
                .as_ref()
                .is_none_or(|looper| looper.finished.load(Ordering::Acquire))
            {
                *slot = None;
            }
            return;
        }
        // HTTP など別経路で演奏・preview が始まった。そちらの音を優先する。
        if *self.playback.play_state.lock().unwrap() != DawPlayState::Idle {
            *slot = None;
            drop(slot);
            self.append_log_line("startup-audition: cancel reason=other-playback");
            return;
        }

        let measure = audition.measure_index + 1;
        let measure_samples = self.measure_duration_samples();
        let track_gains = self.playback_track_gains();
        if audition.looper.is_none() {
            if let Some(cached) = try_get_cached_samples(
                &self.cache,
                measure,
                measure_samples,
                self.editor.tracks,
                &track_gains,
            )
            .filter(|cached| !cached.cached_tracks.is_empty())
            {
                audition.looper = Some(
                    self.spawn_audition_looper(fold_into_loop(&cached.samples, measure_samples)),
                );
                self.append_log_line(format!("startup-audition: loop start meas{measure}"));
            }
        }

        if !audition.server_settled.load(Ordering::Acquire)
            || !measure_render_settled(&self.cache.lock().unwrap(), measure, &track_gains)
        {
            return;
        }
        audition.handed_off = true;
        let looper = audition.looper.clone();
        let measure_index = audition.measure_index;
        drop(slot);

        self.append_log_line(format!("startup-audition: hand off meas{measure}"));
        self.start_play_from_measure(measure_index);
        // 本演奏が Playing になってから渡す。先に渡すと、Idle を見たループが即座に止まる。
        if let Some(looper) = looper {
            looper.handed_off.store(true, Ordering::Release);
        }
    }

    /// 本演奏へ切り替える前の試聴ループを止める。切り替え前だったら `true`。
    ///
    /// 切り替え前は `play_state` が `Idle` なので、Shift+Space をそのまま通すと
    /// 「鳴っているのに演奏開始」になる。呼び出し側は `true` なら停止として扱うこと。
    pub(crate) fn cancel_startup_audition(&self) -> bool {
        let Some(audition) = self.playback.startup_audition.lock().unwrap().take() else {
            return false;
        };
        if audition.handed_off {
            return false;
        }
        self.append_log_line("startup-audition: cancel reason=user");
        true
    }

    /// 試聴ループが鳴っている間は「音が鳴るまで」overlay を出さない（もう鳴っている）。
    pub(crate) fn startup_audition_is_sounding(&self) -> bool {
        self.playback
            .startup_audition
            .lock()
            .unwrap()
            .as_ref()
            .and_then(|audition| audition.looper.as_ref())
            .is_some_and(|looper| !looper.finished.load(Ordering::Acquire))
    }

    fn spawn_audition_looper(&self, samples: Vec<f32>) -> Arc<LooperControl> {
        let control = Arc::new(LooperControl::default());
        let thread_control = Arc::clone(&control);
        let sample_rate = self.cfg.sample_rate as u32;
        let play_state = Arc::clone(&self.playback.play_state);
        let position = Arc::clone(&self.playback.position);
        let log_lines = Arc::clone(&self.log_lines);
        std::thread::spawn(move || {
            run_looper(
                &thread_control,
                samples,
                sample_rate,
                &play_state,
                &position,
                &log_lines,
            );
            thread_control.finished.store(true, Ordering::Release);
        });
        control
    }
}

fn run_looper(
    control: &LooperControl,
    samples: Vec<f32>,
    sample_rate: u32,
    play_state: &Mutex<DawPlayState>,
    position: &Mutex<Option<PlayPosition>>,
    log_lines: &Arc<Mutex<VecDeque<String>>>,
) {
    let Some(rodio_sample_rate) = rodio::SampleRate::new(sample_rate) else {
        crate::append_log_line(log_lines, "startup-audition: sample rate is zero");
        return;
    };
    // device sink を drop すると音が止まるので、ループが終わるまで持つ。
    let Ok(device_sink) = cmrt_tui_core::audio_output::open_default_sink() else {
        crate::append_log_line(log_lines, "startup-audition: audio init failed");
        return;
    };
    let player = rodio::Player::connect_new(device_sink.mixer());
    player.append(
        rodio::buffer::SamplesBuffer::new(
            cmrt_tui_core::playback_session::STEREO,
            rodio_sample_rate,
            samples,
        )
        .repeat_infinite(),
    );
    while !control.should_stop(play_state, position) {
        std::thread::sleep(Duration::from_millis(5));
    }
    player.stop();
    crate::append_log_line(log_lines, "startup-audition: loop stop");
}

#[cfg(test)]
mod tests;
