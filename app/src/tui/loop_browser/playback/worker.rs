use std::collections::HashMap;
use std::sync::{mpsc, Arc, Mutex};
use std::time::{Duration, Instant};

use super::diagnostics::{log_event, path_label, SharedLoopStretchDiagnostics};
use super::gain::effective_track_gain;
use super::position::{self, ScheduledMeasure, SharedPlaybackPosition};
use super::preparation::{profile_label, PreparationWorker, PreparedSet};
use super::sinks::{
    play_path, play_path_profiled, stop_pad_sinks, stop_sinks, take_pad_voice, TrackSink,
};
use super::tempo::{grid_target_bpm, measure_duration, measure_timing};
use super::{LoopPlaybackCommand, LoopPlaybackGrid};
use crate::loop_time_stretch::format_bpm;
use crate::tui::loop_browser::LoopGridChange;
use crate::tui::loop_browser::LoopPlaybackClip;
use crate::tui::PlayState;
use anyhow::{Context, Result};

#[derive(Default)]
pub(super) struct TransportState {
    paused: bool,
    current_measure: Option<usize>,
    restart_measure: Option<usize>,
}

impl TransportState {
    pub(super) fn pause(&mut self) {
        self.paused = true;
        self.current_measure = None;
        self.restart_measure = None;
    }

    pub(super) fn resume_at(&mut self, measure: usize) {
        self.paused = false;
        self.current_measure = None;
        self.restart_measure = Some(measure);
    }

    fn restart_at(&mut self, measure: usize) {
        self.current_measure = None;
        self.restart_measure = Some(measure);
    }

    fn started(&mut self, measure: usize) {
        self.current_measure = Some(measure);
    }

    fn clear_current(&mut self) {
        self.current_measure = None;
    }

    pub(super) fn next_measure_to_start(&mut self, grid: &LoopPlaybackGrid) -> Option<usize> {
        if self.paused {
            return None;
        }
        self.restart_measure
            .take()
            .and_then(|measure| measure_at_or_after(grid, measure))
            .or_else(|| next_measure(grid, self.current_measure))
    }

    pub(super) fn is_paused(&self) -> bool {
        self.paused
    }
}

pub(super) fn playback_worker(
    receiver: mpsc::Receiver<LoopPlaybackCommand>,
    grid: LoopPlaybackGrid,
    mut track_volumes_db: Vec<i32>,
    mut solo_tracks: Vec<bool>,
    state: &Arc<Mutex<PlayState>>,
    diagnostics: SharedLoopStretchDiagnostics,
    playback_position: SharedPlaybackPosition,
) -> Result<()> {
    position::clear(&playback_position);
    let (_stream, handle) =
        rodio::OutputStream::try_default().context("audio output deviceを開けません")?;
    let mut preview_sink: Option<Arc<rodio::Sink>> = None;
    let mut pad_sinks = HashMap::<char, Arc<rodio::Sink>>::new();
    let mut measure_sinks = Vec::<TrackSink>::new();
    let mut measure_deadline = None;
    let mut preparation = PreparationWorker::spawn(diagnostics);
    let initial_target = grid_target_bpm(&grid);
    let mut pending_generation = Some(preparation.submit(grid, LoopGridChange::Initial));
    let mut active: Option<PreparedSet> = None;
    let mut transport = TransportState::default();
    set_play_state(
        state,
        PlayState::Running(format!("BPM{}変換中", format_bpm(initial_target.bpm))),
    );

    loop {
        while let Some(result) = preparation.try_result() {
            if pending_generation != Some(result.generation) {
                continue;
            }
            pending_generation = None;
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
        if measure_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            measure_deadline = None;
            position::clear(&playback_position);
        }
        if !transport.is_paused() && measure_deadline.is_none() {
            if let Some(prepared) = active.as_ref() {
                let next = transport.next_measure_to_start(&prepared.grid);
                if let Some(measure) = next {
                    let start_result =
                        start_measure(&handle, prepared, measure, &track_volumes_db, &solo_tracks);
                    let started_at = Instant::now();
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
                let (result, timing) = play_path_profiled(&handle, &path);
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
                match play_path(&handle, &path) {
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
                let target_bpm = grid_target_bpm(&next_grid);
                pending_generation = Some(preparation.submit(next_grid, reason));
                set_preparation_state(state, transport.is_paused(), target_bpm.bpm);
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
                transport.restart_at(start_measure);
                let target_bpm = grid_target_bpm(&grid);
                pending_generation = Some(preparation.submit(grid, reason));
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

pub(super) fn next_measure(grid: &LoopPlaybackGrid, current: Option<usize>) -> Option<usize> {
    let measures = grid.iter().map(Vec::len).max().unwrap_or(0);
    if measures == 0 {
        return None;
    }
    let start = current.map_or(0, |measure| (measure + 1) % measures);
    for offset in 0..measures {
        let measure = (start + offset) % measures;
        if measure_is_occupied(grid, measure) {
            return Some(measure);
        }
    }
    None
}

pub(super) fn measure_at_or_after(grid: &LoopPlaybackGrid, start: usize) -> Option<usize> {
    let measures = grid.iter().map(Vec::len).max().unwrap_or(0);
    if measures == 0 {
        return None;
    }
    let start = start % measures;
    for offset in 0..measures {
        let measure = (start + offset) % measures;
        if measure_is_occupied(grid, measure) {
            return Some(measure);
        }
    }
    None
}

fn measure_is_occupied(grid: &LoopPlaybackGrid, measure: usize) -> bool {
    grid.iter().any(|track| {
        track
            .iter()
            .take(measure.saturating_add(1))
            .enumerate()
            .any(|(start, clip)| {
                clip.as_ref()
                    .is_some_and(|clip| start.saturating_add(clip.span_measures) > measure)
            })
    })
}

pub(super) fn starting_clips(
    grid: &LoopPlaybackGrid,
    measure: usize,
) -> Vec<(usize, &LoopPlaybackClip)> {
    grid.iter()
        .enumerate()
        .filter_map(|(track, cells)| {
            cells
                .get(measure)
                .and_then(Option::as_ref)
                .map(|clip| (track, clip))
        })
        .collect()
}

fn start_measure(
    handle: &rodio::OutputStreamHandle,
    prepared: &PreparedSet,
    measure: usize,
    track_volumes_db: &[i32],
    solo_tracks: &[bool],
) -> Result<Vec<TrackSink>> {
    let clips = starting_clips(&prepared.grid, measure);
    let mut sinks = Vec::with_capacity(clips.len());
    for (track, clip) in clips {
        let Some(Ok(audio)) = prepared.audio_for(clip) else {
            log_event(format!(
                "event=playback-skip generation={} measure={} track={} path=\"{}\" reason=prepared-audio-unavailable",
                prepared.generation,
                measure + 1,
                track + 1,
                path_label(&clip.path),
            ));
            continue;
        };
        let sink = Arc::new(rodio::Sink::try_new(handle)?);
        sink.set_volume(effective_track_gain(track, track_volumes_db, solo_tracks));
        let info = audio.info();
        let playback_duration = measure_duration(&prepared.grid, prepared.target_bpm.bpm)
            .checked_mul(u32::try_from(clip.span_measures).unwrap_or(u32::MAX))
            .unwrap_or(Duration::MAX);
        let output_frames =
            super::scheduling::append_clip_source(&sink, audio, clip, playback_duration);
        log_event(format!(
            "event=playback-start generation={} measure={} track={} path=\"{}\" source_bpm={} target_bpm={} output_frames={} output_seconds={:.6} cropped={} profile={}",
            prepared.generation,
            measure + 1,
            track + 1,
            path_label(&clip.path),
            clip.source_bpm()
                .map(format_bpm)
                .unwrap_or_else(|| "one-shot".to_string()),
            format_bpm(prepared.target_bpm.bpm),
            output_frames,
            super::diagnostics::duration_seconds(output_frames, info.sample_rate),
            output_frames < info.output_frames,
            super::diagnostics::profile_label(info.profile),
        ));
        sinks.push(TrackSink { track, sink });
    }
    Ok(sinks)
}

fn set_play_state(state: &Arc<Mutex<PlayState>>, next: PlayState) {
    *state.lock().unwrap() = next;
}
