use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use super::diagnostics::{log_event, SharedLoopStretchDiagnostics};
use super::gain::effective_track_gain;
use super::position::{self, ScheduledMeasure, SharedPlaybackPosition};
use super::preparation::{profile_label, PreparationWorker, PreparedSet};
use super::sinks::{
    play_path, play_path_profiled, stop_pad_sinks, stop_sinks, take_pad_voice, TrackSink,
};
use super::tempo::{grid_target_bpm, measure_timing};
use super::{LoopPlaybackCommand, LoopPlaybackGrid};
use crate::LoopGridChange;
use anyhow::{Context, Result};
use cmrt_loop_browser_domain::time_stretch::format_bpm;
use cmrt_tui_core::PlayState;

mod config;
mod measure_start;
mod transport;

pub(super) use config::PlaybackWorkerConfig;
use measure_start::start_measure;
pub use measure_start::starting_clips;
pub use transport::TransportState;
#[cfg(test)]
pub use transport::{measure_at_or_after, next_measure};

/// measure の開始がスケジュールよりこれ以上遅れたらログに出す。
/// rodio/cpal は underrun カウンタを出さないので、worker スレッドの遅れを代理指標にする。
const MEASURE_LATENESS_LOG_THRESHOLD: Duration = Duration::from_millis(5);

pub fn playback_worker(
    receiver: mpsc::Receiver<LoopPlaybackCommand>,
    config: PlaybackWorkerConfig,
    state: &Arc<Mutex<PlayState>>,
    diagnostics: SharedLoopStretchDiagnostics,
    playback_position: SharedPlaybackPosition,
) -> Result<()> {
    let PlaybackWorkerConfig {
        grid,
        mut track_volumes_db,
        mut solo_tracks,
        mut bpm_mode,
    } = config;
    position::clear(&playback_position);
    // device sink を drop すると全 Player の再生が止まるため、worker が生きている間は保持する。
    let device_sink =
        rodio::DeviceSinkBuilder::open_default_sink().context("audio output deviceを開けません")?;
    let handle = device_sink.mixer();
    let mut preview_sink: Option<Arc<rodio::Player>> = None;
    let mut pad_sinks = HashMap::<char, Arc<rodio::Player>>::new();
    let mut measure_sinks = Vec::<TrackSink>::new();
    let mut measure_deadline = None;
    let mut preparation = PreparationWorker::spawn(diagnostics);
    let initial_target = grid_target_bpm(&grid, bpm_mode);
    let mut pending_generation = Some(preparation.submit(grid, LoopGridChange::Initial, bpm_mode));
    let mut active: Option<PreparedSet> = None;
    // 先読み（オートランダム）で用意した次のグリッド。周の境目でだけ active へ差し替える。
    let mut standby: Option<PreparedSet> = None;
    let mut preload_generation: Option<u64> = None;
    let mut standby_token = 0u64;
    // 先読みを準備したときのテンポ。差し替えが成立したときだけ現行テンポへ昇格させる。
    let mut standby_bpm_mode = bpm_mode;
    let mut active_token = 0u64;
    let mut transport = TransportState::default();
    set_play_state(
        state,
        PlayState::Running(format!("BPM{}変換中", format_bpm(initial_target.bpm))),
    );

    loop {
        while let Some(result) = preparation.try_result() {
            // 先読みの結果は standby に置くだけ。active も PlayState も触らず、演奏を続ける。
            if preload_generation == Some(result.generation) {
                preload_generation = None;
                standby = Some(result.prepared);
                continue;
            }
            if pending_generation != Some(result.generation) {
                continue;
            }
            pending_generation = None;
            active_token = 0;
            active = Some(result.prepared);
            let prepared = active.as_ref().expect("just assigned");
            if transport.is_paused() {
                set_play_state(state, PlayState::Idle);
            } else if let Some(warning) = &prepared.warning {
                set_play_state(state, PlayState::Err(warning.clone()));
            } else if prepared.grid.iter().flatten().any(Option::is_some) {
                set_play_state(
                    state,
                    PlayState::Playing(format!(
                        "BPM{} 準備完了",
                        format_bpm(prepared.target_bpm.bpm)
                    )),
                );
            } else {
                set_play_state(state, PlayState::Idle);
            }
        }
        pad_sinks.retain(|_, sink| !sink.empty());
        measure_sinks.retain(|voice| !voice.sink.empty());
        let mut expired_deadline = None;
        if let Some(deadline) = measure_deadline {
            if Instant::now() >= deadline {
                measure_deadline = None;
                expired_deadline = Some(deadline);
                position::clear(&playback_position);
            }
        }
        // 先読みが間に合っていて、いま周の境目なら差し替える。
        // 間に合っていなければ何もしない＝音を止めずに現行グリッドをもう1周させる。
        if !transport.is_paused()
            && measure_deadline.is_none()
            && standby.is_some()
            && active
                .as_ref()
                .is_some_and(|prepared| transport.wraps_next(&prepared.grid))
        {
            let prepared = standby.take().expect("standby.is_some() を確認済み");
            log_event(format!(
                "event=grid-swapped token={standby_token} generation={} target_bpm={}",
                prepared.generation,
                format_bpm(prepared.target_bpm.bpm),
            ));
            preparation.promote_background(&prepared);
            active = Some(prepared);
            active_token = standby_token;
            // 差し替えが成立して初めて、先読みを準備したテンポが現行テンポになる。
            bpm_mode = standby_bpm_mode;
            transport.clear_current();
            transport.reset_cycle();
        }
        if !transport.is_paused() && measure_deadline.is_none() {
            if let Some(prepared) = active.as_ref() {
                let next = transport.next_measure_to_start(&prepared.grid);
                if let Some(measure) = next {
                    let start_result =
                        start_measure(handle, prepared, measure, &track_volumes_db, &solo_tracks);
                    let started_at = Instant::now();
                    log_measure_lateness(
                        expired_deadline,
                        started_at,
                        measure,
                        &preload_generation,
                    );
                    let timing = measure_timing(&prepared.grid, prepared.target_bpm.bpm);
                    transport.started(measure);
                    measure_deadline = Some(started_at + timing.duration);
                    position::set(
                        &playback_position,
                        ScheduledMeasure::new(
                            measure,
                            started_at,
                            timing.duration,
                            timing.beats_per_measure,
                            transport.cycle(),
                            active_token,
                        ),
                    );
                    match start_result {
                        Ok(sinks) => {
                            measure_sinks.extend(sinks);
                            if pending_generation.is_none() && prepared.warning.is_none() {
                                let profile = starting_clips(&prepared.grid, measure)
                                    .first()
                                    .map_or("-", |(_, clip)| profile_label(clip));
                                set_play_state(
                                    state,
                                    PlayState::Playing(format!(
                                        "BPM{} loop measure {} {profile}",
                                        format_bpm(prepared.target_bpm.bpm),
                                        measure + 1
                                    )),
                                );
                            }
                        }
                        Err(error) => {
                            set_play_state(
                                state,
                                PlayState::Err(format!("WAV loop再生に失敗: {error}")),
                            );
                        }
                    }
                } else {
                    transport.clear_current();
                    position::clear(&playback_position);
                    if pending_generation.is_none() && prepared.warning.is_none() {
                        set_play_state(state, PlayState::Idle);
                    }
                }
            }
        }

        let mut timeout = measure_deadline
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .unwrap_or(Duration::from_secs(60));
        if pending_generation.is_some() {
            timeout = timeout.min(Duration::from_millis(20));
        }
        match receiver.recv_timeout(timeout) {
            Ok(LoopPlaybackCommand::Preview {
                path,
                trace_id,
                queued_at,
            }) => {
                let queue = queued_at.elapsed();
                let processing_started = Instant::now();
                if let Some(sink) = preview_sink.take() {
                    sink.stop();
                }
                let (result, timing) = play_path_profiled(handle, &path);
                let outcome = match result {
                    Ok(sink) => {
                        preview_sink = Some(sink);
                        "ok"
                    }
                    Err(error) => {
                        set_play_state(state, PlayState::Err(format!("WAV再生に失敗: {error}")));
                        "error"
                    }
                };
                super::super::performance::log_preview_finished(
                    super::super::performance::PreviewMetrics {
                        trace_id,
                        queue,
                        open: timing.open,
                        decode: timing.decode,
                        sink: timing.sink,
                        append: timing.append,
                        total: queue.saturating_add(processing_started.elapsed()),
                        path: &path,
                        outcome,
                    },
                );
            }
            Ok(LoopPlaybackCommand::Trigger { pad, path }) => {
                if let Some(sink) = preview_sink.take() {
                    sink.stop();
                }
                if let Some(sink) = take_pad_voice(&mut pad_sinks, pad) {
                    sink.stop();
                }
                match play_path(handle, &path) {
                    Ok(sink) => {
                        pad_sinks.insert(pad, sink);
                    }
                    Err(error) => {
                        set_play_state(state, PlayState::Err(format!("WAV pad再生に失敗: {error}")))
                    }
                }
            }
            Ok(LoopPlaybackCommand::SetGrid {
                grid: next_grid,
                reason,
            }) => {
                standby = None;
                preload_generation = None;
                standby_token = 0;
                let target_bpm = grid_target_bpm(&next_grid, bpm_mode);
                pending_generation = Some(preparation.submit(next_grid, reason, bpm_mode));
                set_preparation_state(state, transport.is_paused(), target_bpm.bpm);
            }
            Ok(LoopPlaybackCommand::PreloadGrid {
                grid,
                token,
                mode,
                reason,
            }) => {
                // 鳴っていないとき・前景の準備中は先読みしない。
                // UI 側が周回数を見て予約を捨て、次の周で引き直す。
                if transport.is_paused() || active.is_none() || pending_generation.is_some() {
                    log_event(format!(
                        "event=preload-rejected token={token} paused={} active={} preparing={}",
                        transport.is_paused(),
                        active.is_some(),
                        pending_generation.is_some(),
                    ));
                } else {
                    standby = None;
                    standby_token = token;
                    standby_bpm_mode = mode;
                    preload_generation = Some(preparation.submit_background(grid, reason, mode));
                }
            }
            Ok(LoopPlaybackCommand::RestartGridAt {
                grid,
                start_measure,
                reason,
            }) => {
                stop_sinks(&mut measure_sinks);
                stop_pad_sinks(&mut pad_sinks);
                if let Some(sink) = preview_sink.take() {
                    sink.stop();
                }
                measure_deadline = None;
                position::clear(&playback_position);
                active = None;
                active_token = 0;
                standby = None;
                preload_generation = None;
                standby_token = 0;
                transport.restart_at(start_measure);
                let target_bpm = grid_target_bpm(&grid, bpm_mode);
                pending_generation = Some(preparation.submit(grid, reason, bpm_mode));
                set_preparation_state(state, transport.is_paused(), target_bpm.bpm);
            }
            Ok(LoopPlaybackCommand::ReplaceTrackLayout {
                grid,
                start_measure,
                track_volumes_db: next_track_volumes_db,
                solo_tracks: next_solo_tracks,
            }) => {
                stop_sinks(&mut measure_sinks);
                stop_pad_sinks(&mut pad_sinks);
                if let Some(sink) = preview_sink.take() {
                    sink.stop();
                }
                measure_deadline = None;
                position::clear(&playback_position);
                active = None;
                active_token = 0;
                standby = None;
                preload_generation = None;
                standby_token = 0;
                track_volumes_db = next_track_volumes_db;
                solo_tracks = next_solo_tracks;
                transport.restart_at(start_measure);
                let target_bpm = grid_target_bpm(&grid, bpm_mode);
                pending_generation =
                    Some(preparation.submit(grid, LoopGridChange::TrackOrder, bpm_mode));
                set_preparation_state(state, transport.is_paused(), target_bpm.bpm);
            }
            Ok(LoopPlaybackCommand::SetTrackVolume { track, volume_db }) => {
                if track >= track_volumes_db.len() {
                    track_volumes_db.resize(track + 1, 0);
                }
                track_volumes_db[track] = volume_db;
                let gain = effective_track_gain(track, &track_volumes_db, &solo_tracks);
                for voice in measure_sinks.iter().filter(|voice| voice.track == track) {
                    voice.sink.set_volume(gain);
                }
            }
            Ok(LoopPlaybackCommand::SetTrackSolo {
                solo_tracks: next_solo_tracks,
            }) => {
                solo_tracks = next_solo_tracks;
                for voice in &measure_sinks {
                    voice.sink.set_volume(effective_track_gain(
                        voice.track,
                        &track_volumes_db,
                        &solo_tracks,
                    ));
                }
            }
            Ok(LoopPlaybackCommand::Pause) => {
                standby = None;
                preload_generation = None;
                standby_token = 0;
                transport.pause();
                stop_sinks(&mut measure_sinks);
                stop_pad_sinks(&mut pad_sinks);
                if let Some(sink) = preview_sink.take() {
                    sink.stop();
                }
                measure_deadline = None;
                position::clear(&playback_position);
                set_play_state(state, PlayState::Idle);
            }
            Ok(LoopPlaybackCommand::ResumeAt(start_measure)) => {
                transport.resume_at(start_measure);
                measure_deadline = None;
                if pending_generation.is_some() {
                    set_play_state(state, PlayState::Running("WAV loop変換中".to_string()));
                }
            }
            Ok(LoopPlaybackCommand::SetBpmMode { mode, grid }) => {
                stop_sinks(&mut measure_sinks);
                stop_pad_sinks(&mut pad_sinks);
                if let Some(sink) = preview_sink.take() {
                    sink.stop();
                }
                measure_deadline = None;
                position::clear(&playback_position);
                active = None;
                active_token = 0;
                standby = None;
                preload_generation = None;
                standby_token = 0;
                bpm_mode = mode;
                standby_bpm_mode = mode;
                transport.restart_at(0);
                let target_bpm = grid_target_bpm(&grid, bpm_mode);
                pending_generation =
                    Some(preparation.submit(grid, LoopGridChange::Tempo, bpm_mode));
                set_preparation_state(state, transport.is_paused(), target_bpm.bpm);
            }
            Ok(LoopPlaybackCommand::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                preparation.cancel();
                position::clear(&playback_position);
                stop_sinks(&mut measure_sinks);
                stop_pad_sinks(&mut pad_sinks);
                if let Some(sink) = preview_sink.take() {
                    sink.stop();
                }
                return Ok(());
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
}

/// measure の開始がスケジュールから遅れた量を記録する。
/// 先読み中に遅れが出るなら、裏の準備が演奏を押しのけている疑いがある。
fn log_measure_lateness(
    expired_deadline: Option<Instant>,
    started_at: Instant,
    measure: usize,
    preload_generation: &Option<u64>,
) {
    let Some(deadline) = expired_deadline else {
        return;
    };
    let lateness = started_at.saturating_duration_since(deadline);
    if lateness < MEASURE_LATENESS_LOG_THRESHOLD {
        return;
    }
    log_event(format!(
        "event=measure-late measure={} lateness_ms={} preloading={}",
        measure + 1,
        lateness.as_millis(),
        preload_generation.is_some(),
    ));
}

fn set_preparation_state(state: &Arc<Mutex<PlayState>>, paused: bool, bpm: f64) {
    if paused {
        set_play_state(state, PlayState::Idle);
    } else {
        set_play_state(
            state,
            PlayState::Running(format!("BPM{}変換中", format_bpm(bpm))),
        );
    }
}

fn set_play_state(state: &Arc<Mutex<PlayState>>, next: PlayState) {
    *state.lock().unwrap() = next;
}
