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

pub(crate) mod progress;

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
    progress: Arc<progress::Progress>,
    render_settled_logged: bool,
}

impl StartupAudition {
    pub(crate) fn playback_progress(&self) -> Option<Arc<progress::Progress>> {
        self.handed_off.then(|| Arc::clone(&self.progress))
    }

    pub(crate) fn new(measure_index: usize, server_settled: Arc<AtomicBool>) -> Self {
        Self {
            measure_index,
            server_settled,
            looper: None,
            handed_off: false,
            progress: Arc::new(progress::Progress::default()),
            render_settled_logged: false,
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
        self.append_log_line("起動時自動演奏: 自動演奏を要求しました。");
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
        let audition = StartupAudition::new(measure_index, Arc::clone(&server_settled));
        let progress = Arc::clone(&audition.progress);
        progress.log(&self.log_lines, format!("準備を開始しました。開始小節=M{}。演奏サーバーの起動と開始小節のキャッシュ生成を待ち、準備が整うと自動で演奏します。", measure_index + 1));
        progress.log(
            &self.log_lines,
            "キャッシュWAVが利用可能になれば、待機中に簡易ループ演奏します。",
        );
        let settled = Arc::clone(&server_settled);
        let log_lines = Arc::clone(&self.log_lines);
        std::thread::spawn(move || {
            if let Err(error) = play_server.ensure_started_for_fast_midi() {
                progress.log(
                    &log_lines,
                    format!("演奏サーバーの起動に失敗しました。詳細={error:#}"),
                );
                crate::append_log_line(
                    &log_lines,
                    format!("startup-audition: server start failed error=\"{error:#}\""),
                );
            } else {
                progress.log(&log_lines, "演奏サーバーの起動が完了しました。");
            }
            settled.store(true, Ordering::Release);
        });

        self.append_log_line(format!("startup-audition: begin meas{}", measure_index + 1));
        *self.playback.startup_audition.lock().unwrap() = Some(audition);
    }

    /// メインループが毎 tick 呼ぶ。ループの開始と、本演奏への切り替えを進める。
    pub(crate) fn pump_startup_audition(&self) {
        let mut slot = self.playback.startup_audition.lock().unwrap();
        let Some(audition) = slot.as_mut() else {
            return;
        };
        if audition.handed_off {
            let finished = audition.progress.poll(
                &self.log_lines,
                *self.playback.play_state.lock().unwrap() == DawPlayState::Playing,
                Instant::now(),
            );
            if finished
                && audition
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
            audition
                .progress
                .cancel(&self.log_lines, "別の演奏・プレビューが開始されました");
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

        let cache = self.cache.lock().unwrap();
        let render_settled = measure_render_settled(&cache, measure, &track_gains);
        if render_settled && !audition.render_settled_logged {
            let errors = (FIRST_PLAYABLE_TRACK..cache.len())
                .filter(|&track| track_gains.get(track).copied().unwrap_or(1.0) != 0.0)
                .filter(|&track| cache[track][measure].state == CacheState::Error)
                .count();
            audition.progress.log(
                &self.log_lines,
                format!(
                    "開始小節 M{measure} のキャッシュ生成待ちが終了しました。生成エラー={errors}"
                ),
            );
        }
        audition.render_settled_logged = render_settled;
        drop(cache);
        if !audition.server_settled.load(Ordering::Acquire) || !render_settled {
            return;
        }
        audition.progress.log(
            &self.log_lines,
            "本演奏への切り替えを開始します。開始小節のWAVロードと演奏開始を待っています。",
        );
        if audition.looper.is_none() {
            audition.progress.log(
                &self.log_lines,
                "利用可能なキャッシュWAVがないため、簡易ループ演奏なしで切り替えます。",
            );
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
        audition.progress.cancel(&self.log_lines, "停止・取消操作");
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
        crate::append_log_line(
            log_lines,
            "起動時自動演奏: 簡易ループ演奏を開始できません。サンプルレートが0です。",
        );
        crate::append_log_line(log_lines, "startup-audition: sample rate is zero");
        return;
    };
    // device sink を drop すると音が止まるので、ループが終わるまで持つ。
    let Ok(device_sink) = cmrt_tui_core::audio_output::open_default_sink() else {
        crate::append_log_line(log_lines, "起動時自動演奏: 音声出力の初期化に失敗したため、簡易ループ演奏を開始できません。本演奏の準備を続けます。");
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
    crate::append_log_line(
        log_lines,
        "起動時自動演奏: キャッシュWAVの簡易ループ演奏を開始しました。",
    );
    while !control.should_stop(play_state, position) {
        std::thread::sleep(Duration::from_millis(5));
    }
    player.stop();
    crate::append_log_line(log_lines, "起動時自動演奏: 簡易ループ演奏を停止しました。");
    crate::append_log_line(log_lines, "startup-audition: loop stop");
}

#[cfg(test)]
mod tests;
