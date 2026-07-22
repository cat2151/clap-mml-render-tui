use std::path::PathBuf;
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Instant;

#[cfg(test)]
use super::LoopPlaybackClip;
use super::{LoopGridChange, LoopPlaybackGrid};
use crate::tui::{Mode, PlayState, TuiApp};

pub(in crate::tui) mod diagnostics;
mod gain;
pub(in crate::tui) mod position;
mod preparation;
mod scheduling;
mod sinks;
mod tempo;
mod worker;

#[cfg(test)]
use sinks::take_pad_voice;
#[cfg(test)]
use tempo::{grid_target_bpm, measure_duration, measure_timing};
use worker::playback_worker;
#[cfg(test)]
use worker::{measure_at_or_after, next_measure, starting_clips, TransportState};

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
    SetTrackVolume {
        track: usize,
        volume_db: i32,
    },
    SetTrackSolo {
        solo_tracks: Vec<bool>,
    },
    Pause,
    ResumeAt(usize),
    Stop,
}

pub(in crate::tui) struct LoopPlaybackController {
    sender: mpsc::Sender<LoopPlaybackCommand>,
    worker: Option<JoinHandle<()>>,
}

impl LoopPlaybackController {
    fn spawn(
        grid: LoopPlaybackGrid,
        track_volumes_db: Vec<i32>,
        solo_tracks: Vec<bool>,
        state: Arc<Mutex<PlayState>>,
        diagnostics: diagnostics::SharedLoopStretchDiagnostics,
        playback_position: position::SharedPlaybackPosition,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            if let Err(error) = playback_worker(
                receiver,
                grid,
                track_volumes_db,
                solo_tracks,
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

    pub(super) fn preview(&self, path: PathBuf, trace_id: u64) {
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

    pub(super) fn trigger(&self, pad: char, path: PathBuf) {
        let _ = self.sender.send(LoopPlaybackCommand::Trigger { pad, path });
    }

    pub(super) fn set_grid(&self, grid: LoopPlaybackGrid, reason: LoopGridChange) {
        let _ = self
            .sender
            .send(LoopPlaybackCommand::SetGrid { grid, reason });
    }

    pub(super) fn restart_grid_at(
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

    pub(super) fn set_track_volume(&self, track: usize, volume_db: i32) {
        let _ = self
            .sender
            .send(LoopPlaybackCommand::SetTrackVolume { track, volume_db });
    }

    pub(super) fn set_track_solo(&self, solo_tracks: Vec<bool>) {
        let _ = self
            .sender
            .send(LoopPlaybackCommand::SetTrackSolo { solo_tracks });
    }

    pub(super) fn set_paused(&self, paused: bool, start_measure: usize) {
        let command = if paused {
            LoopPlaybackCommand::Pause
        } else {
            LoopPlaybackCommand::ResumeAt(start_measure)
        };
        let _ = self.sender.send(command);
    }

    fn stop(&mut self) {
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

impl<'a> TuiApp<'a> {
    pub(in crate::tui) fn begin_loop_browser_startup(&mut self) {
        self.mode = Mode::LoopBrowser;
        self.loop_browser.state.starting = true;
    }

    pub(in crate::tui) fn complete_loop_browser_startup(&mut self) {
        if !self.loop_browser.state.starting {
            return;
        }
        let session = self.begin_playback_session();
        self.set_play_state_if_current(session, PlayState::Idle);
        self.loop_browser.state.reload(&self.cfg);
        if !cfg!(test) {
            self.loop_browser.playback = Some(LoopPlaybackController::spawn(
                self.loop_browser.state.playback_grid(),
                self.loop_browser.state.track_volumes_db().to_vec(),
                self.loop_browser.state.solo_tracks().to_vec(),
                Arc::clone(&self.playback.play_state),
                Arc::clone(&self.loop_browser.state.stretch_diagnostics),
                Arc::clone(&self.loop_browser.state.playback_position),
            ));
        }
    }

    pub(in crate::tui) fn finish_loop_browser(&mut self) {
        self.loop_browser.state.starting = false;
        if let Some(mut playback) = self.loop_browser.playback.take() {
            playback.stop();
        }
        let session = self.begin_playback_session();
        self.set_play_state_if_current(session, PlayState::Idle);
        self.mode = Mode::Normal;
    }

    pub(in crate::tui) fn preview_loop_file(&self, path: PathBuf, trace_id: u64) {
        if let Some(playback) = &self.loop_browser.playback {
            playback.preview(path, trace_id);
        }
    }

    pub(in crate::tui) fn trigger_loop_pad(&self, pad: char, path: PathBuf) {
        if let Some(playback) = &self.loop_browser.playback {
            playback.trigger(pad, path);
        }
    }

    pub(in crate::tui) fn update_loop_grid(&self, grid: LoopPlaybackGrid, reason: LoopGridChange) {
        if let Some(playback) = &self.loop_browser.playback {
            playback.set_grid(grid, reason);
        }
    }

    pub(in crate::tui) fn restart_loop_grid_at(
        &self,
        grid: LoopPlaybackGrid,
        start_measure: usize,
        reason: LoopGridChange,
    ) {
        if let Some(playback) = &self.loop_browser.playback {
            playback.restart_grid_at(grid, start_measure, reason);
        }
    }

    pub(in crate::tui) fn update_loop_track_volume(&self, track: usize, volume_db: i32) {
        if let Some(playback) = &self.loop_browser.playback {
            playback.set_track_volume(track, volume_db);
        }
    }

    pub(in crate::tui) fn update_loop_track_solo(&self, solo_tracks: Vec<bool>) {
        if let Some(playback) = &self.loop_browser.playback {
            playback.set_track_solo(solo_tracks);
        }
    }

    pub(in crate::tui) fn set_loop_playback_paused(&self, paused: bool, start_measure: usize) {
        if let Some(playback) = &self.loop_browser.playback {
            playback.set_paused(paused, start_measure);
        }
        if paused {
            *self.playback.play_state.lock().unwrap() = PlayState::Idle;
        }
    }
}

fn set_play_state(state: &Arc<Mutex<PlayState>>, next: PlayState) {
    *state.lock().unwrap() = next;
}

#[cfg(test)]
mod tests;
