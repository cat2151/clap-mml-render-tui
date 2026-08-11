use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

#[cfg(test)]
use super::LoopPlaybackClip;
use super::{LoopGridChange, LoopPlaybackGrid};
use cmrt_tui_core::bpm::BpmMode;
use cmrt_tui_core::PlayState;

pub mod diagnostics;
mod gain;
pub mod position;
mod preparation;
mod scheduling;
mod sinks;
mod tempo;
mod worker;

#[cfg(test)]
use sinks::take_pad_voice;
#[cfg(test)]
use tempo::{grid_target_bpm, measure_duration, measure_timing};
#[cfg(test)]
use worker::{measure_at_or_after, next_measure, starting_clips, TransportState};
use worker::{playback_worker, PlaybackWorkerConfig};

enum LoopPlaybackCommand {
    Preview {
        path: PathBuf,
        trace_id: u64,
        queued_at: Instant,
    },
    Trigger {
        pad: char,
        path: PathBuf,
    },
    SetGrid {
        grid: LoopPlaybackGrid,
        reason: LoopGridChange,
    },
    RestartGridAt {
        grid: LoopPlaybackGrid,
        start_measure: usize,
        reason: LoopGridChange,
    },
    /// 演奏を止めずに裏で次のグリッドを準備し、周の境目で差し替える（オートランダム用）。
    PreloadGrid {
        grid: LoopPlaybackGrid,
        token: u64,
        reason: LoopGridChange,
    },
    ReplaceTrackLayout {
        grid: LoopPlaybackGrid,
        start_measure: usize,
        track_volumes_db: Vec<i32>,
        solo_tracks: Vec<bool>,
    },
    SetTrackVolume {
        track: usize,
        volume_db: i32,
    },
    SetTrackSolo {
        solo_tracks: Vec<bool>,
    },
    Pause,
    ResumeAt(usize),
    SetBpmMode {
        mode: BpmMode,
        grid: LoopPlaybackGrid,
    },
    Stop,
}

pub struct LoopPlaybackController {
    sender: mpsc::Sender<LoopPlaybackCommand>,
    worker: Option<JoinHandle<()>>,
}

impl LoopPlaybackController {
    pub fn spawn(
        grid: LoopPlaybackGrid,
        track_volumes_db: Vec<i32>,
        solo_tracks: Vec<bool>,
        state: Arc<Mutex<PlayState>>,
        diagnostics: diagnostics::SharedLoopStretchDiagnostics,
        playback_position: position::SharedPlaybackPosition,
        bpm_mode: BpmMode,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            if let Err(error) = playback_worker(
                receiver,
                PlaybackWorkerConfig {
                    grid,
                    track_volumes_db,
                    solo_tracks,
                    bpm_mode,
                },
                &state,
                diagnostics,
                playback_position,
            ) {
                set_play_state(
                    &state,
                    PlayState::Err(format!("WAV loop再生に失敗: {error}")),
                );
            }
        });
        Self {
            sender,
            worker: Some(worker),
        }
    }

    pub fn preview(&self, path: PathBuf, trace_id: u64) {
        let started_at = Instant::now();
        let command = LoopPlaybackCommand::Preview {
            path: path.clone(),
            trace_id,
            queued_at: Instant::now(),
        };
        let outcome = if self.sender.send(command).is_ok() {
            "queued"
        } else {
            "worker-stopped"
        };
        super::performance::log_preview_enqueued(trace_id, started_at.elapsed(), &path, outcome);
    }

    pub fn trigger(&self, pad: char, path: PathBuf) {
        let _ = self.sender.send(LoopPlaybackCommand::Trigger { pad, path });
    }

    pub fn set_grid(&self, grid: LoopPlaybackGrid, reason: LoopGridChange) {
        let _ = self
            .sender
            .send(LoopPlaybackCommand::SetGrid { grid, reason });
    }

    pub fn restart_grid_at(
        &self,
        grid: LoopPlaybackGrid,
        start_measure: usize,
        reason: LoopGridChange,
    ) {
        let _ = self.sender.send(LoopPlaybackCommand::RestartGridAt {
            grid,
            start_measure,
            reason,
        });
    }

    pub fn preload_grid(&self, grid: LoopPlaybackGrid, token: u64, reason: LoopGridChange) {
        let _ = self.sender.send(LoopPlaybackCommand::PreloadGrid {
            grid,
            token,
            reason,
        });
    }

    pub fn replace_track_layout(
        &self,
        grid: LoopPlaybackGrid,
        start_measure: usize,
        track_volumes_db: Vec<i32>,
        solo_tracks: Vec<bool>,
    ) {
        let _ = self.sender.send(LoopPlaybackCommand::ReplaceTrackLayout {
            grid,
            start_measure,
            track_volumes_db,
            solo_tracks,
        });
    }

    pub fn set_track_volume(&self, track: usize, volume_db: i32) {
        let _ = self
            .sender
            .send(LoopPlaybackCommand::SetTrackVolume { track, volume_db });
    }

    pub fn set_track_solo(&self, solo_tracks: Vec<bool>) {
        let _ = self
            .sender
            .send(LoopPlaybackCommand::SetTrackSolo { solo_tracks });
    }

    pub fn set_paused(&self, paused: bool, start_measure: usize) {
        let command = if paused {
            LoopPlaybackCommand::Pause
        } else {
            LoopPlaybackCommand::ResumeAt(start_measure)
        };
        let _ = self.sender.send(command);
    }

    pub fn set_bpm_mode(&self, mode: BpmMode, grid: LoopPlaybackGrid) {
        let _ = self
            .sender
            .send(LoopPlaybackCommand::SetBpmMode { mode, grid });
    }

    pub fn stop(&mut self) {
        let _ = self.sender.send(LoopPlaybackCommand::Stop);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

impl Drop for LoopPlaybackController {
    fn drop(&mut self) {
        self.stop();
    }
}

fn set_play_state(state: &Arc<Mutex<PlayState>>, next: PlayState) {
    *state.lock().unwrap() = next;
}

#[cfg(test)]
mod tests;
