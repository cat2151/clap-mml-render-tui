//! DawApp の演奏メソッド
//!
//! 音を出す先は play server だけ。かつてあった rodio 経路（`InProcess` backend、
//! 小節ごとの render キャッシュを自プロセスで mix して sink へ append する）は撤去した。
//! あちらは gain を mix 時に振幅へ焼き込むため、mixer の音量変更が
//! 実測 1 小節（約 2.4 秒）遅れていた。

use std::{
    sync::atomic::Ordering,
    sync::Arc,
    time::{Duration, Instant},
};

use cmrt_runtime::RealtimeAudioBackend;

mod live_cache;
pub(crate) mod live_gain;
mod measure_math;
mod play_server;
#[cfg(test)]
mod real_server;

use super::playback_util::play_start_log_lines;
pub(super) use super::playback_util::{effective_measure_count, loop_measure_summary_label};
use super::{DawApp, DawPlayState, PlayPosition, FIRST_PLAYABLE_TRACK};
pub(super) use measure_math::{current_play_measure_index, following_measure_index};
use measure_math::{format_playback_measure_advance_log, format_playback_measure_resolution_log};

/// 演奏できる中身があるか（1 小節でも空でない MML があるか）。
///
/// 「鳴らすものが無い」と「音を出す先（play server）が用意できていない」は原因が別物で、
/// HTTP API はこの 2 つを別のメッセージで返す。判定規則を 1 か所に置いて食い違いを防ぐ。
fn measures_have_playable_mml(measure_mmls: &[String]) -> bool {
    measure_mmls.iter().any(|mml| !mml.trim().is_empty())
}

fn measure_duration(sample_count: usize, sample_rate: u32) -> std::time::Duration {
    // sample_count はステレオのインターリーブ済みサンプル総数（L/R の合計要素数）。
    // そのため実時間は frames (= sample_count / 2) / sample_rate となり、
    // sample_count / (sample_rate * 2) と等価になる。
    std::time::Duration::from_secs_f64(sample_count as f64 / (sample_rate as f64 * 2.0))
}

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

impl DawApp {
    // ─── 演奏 ─────────────────────────────────────────────────

    /// 演奏できる中身があるか。HTTP API が「鳴らすものが無い」を切り分けるのに使う。
    pub(super) fn has_playable_measures(&self) -> bool {
        measures_have_playable_mml(&self.build_measure_mmls())
    }

    pub(super) fn start_play(&self) {
        self.start_play_from_measure(0);
    }

    pub(super) fn start_play_from_measure(&self, start_measure_index: usize) {
        self.stop_mml_overlay_sender();
        let measure_mmls = self.build_measure_mmls();
        let measure_track_mmls = self.build_measure_track_mmls();
        if !measures_have_playable_mml(&measure_mmls) {
            return;
        }

        // play 状態を最新の値で更新してからスレッドに共有する
        *self.playback.measure_mmls.lock().unwrap() = measure_mmls;
        *self.playback.measure_track_mmls.lock().unwrap() = measure_track_mmls;
        *self.playback.measure_samples.lock().unwrap() = self.measure_duration_samples();

        *self.playback.play_state.lock().unwrap() = DawPlayState::Playing;
        self.append_log_line("play: start");
        for line in play_start_log_lines(
            &self.playback.measure_mmls.lock().unwrap(),
            self.ab_repeat_state(),
        ) {
            self.append_log_line(line);
        }

        // backend ごとに音を出す先が違う。どちらも play server 側で鳴らすので、
        // mixer の gain は演奏スレッドではなく live mix の直前で掛かる。
        match self.cfg.realtime_audio_backend {
            RealtimeAudioBackend::PlayServer => {
                self.start_play_from_measure_via_play_server(start_measure_index)
            }
            RealtimeAudioBackend::CachePlayer => {
                self.start_play_from_measure_via_cache_player(start_measure_index)
            }
        }
    }

    pub(super) fn stop_play(&self) {
        self.stop_mml_overlay_sender();
        let _transition_guard = self.playback.transition_lock.lock().unwrap();
        let prev_state = {
            let mut play_state = self.playback.play_state.lock().unwrap();
            let prev_state = *play_state;
            *play_state = DawPlayState::Idle;
            prev_state
        };
        match prev_state {
            DawPlayState::Idle => {}
            DawPlayState::Preview => {
                self.playback.preview_session.fetch_add(1, Ordering::AcqRel);
                if let Some(sink) = self.playback.preview_sink.lock().unwrap().take() {
                    sink.stop();
                }
                if let Some(play_server) = &self.playback.realtime_play_server {
                    let _ = play_server.stop();
                }
                self.append_log_line("preview: stop");
            }
            DawPlayState::Playing => {
                if let Some(play_server) = &self.playback.realtime_play_server {
                    let _ = play_server.stop();
                }
                self.append_log_line("play: stop");
            }
        }
        *self.playback.position.lock().unwrap() = None;
    }
}

#[cfg(test)]
mod tests;
