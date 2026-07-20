use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::{LoopPlaybackClip, LoopPlaybackGrid};
use crate::loop_time_stretch::TARGET_BPM;
use crate::tui::{Mode, PlayState, TuiApp};
use anyhow::{Context, Result};

mod preparation;

use preparation::{profile_label, PreparationWorker, PreparedSet};

enum LoopPlaybackCommand {
    Preview(PathBuf),
    Trigger { pad: char, path: PathBuf },
    SetGrid(LoopPlaybackGrid),
    SetTrackVolume { track: usize, volume_db: i32 },
    Stop,
}

struct TrackSink {
    track: usize,
    sink: Arc<rodio::Sink>,
}

pub(in crate::tui) struct LoopPlaybackController {
    sender: mpsc::Sender<LoopPlaybackCommand>,
    worker: Option<JoinHandle<()>>,
}

impl LoopPlaybackController {
    fn spawn(
        grid: LoopPlaybackGrid,
        track_volumes_db: Vec<i32>,
        state: Arc<Mutex<PlayState>>,
    ) -> Self {
        let (sender, receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            if let Err(error) = playback_worker(receiver, grid, track_volumes_db, &state) {
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

    pub(super) fn preview(&self, path: PathBuf) {
        let _ = self.sender.send(LoopPlaybackCommand::Preview(path));
    }

    pub(super) fn trigger(&self, pad: char, path: PathBuf) {
        let _ = self.sender.send(LoopPlaybackCommand::Trigger { pad, path });
    }

    pub(super) fn set_grid(&self, grid: LoopPlaybackGrid) {
        let _ = self.sender.send(LoopPlaybackCommand::SetGrid(grid));
    }

    pub(super) fn set_track_volume(&self, track: usize, volume_db: i32) {
        let _ = self
            .sender
            .send(LoopPlaybackCommand::SetTrackVolume { track, volume_db });
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
    pub(in crate::tui) fn start_loop_browser(&mut self) {
        let session = self.begin_playback_session();
        self.set_play_state_if_current(session, PlayState::Idle);
        self.loop_browser.reload(&self.cfg);
        self.mode = Mode::LoopBrowser;
        if !cfg!(test) {
            self.loop_playback = Some(LoopPlaybackController::spawn(
                self.loop_browser.playback_grid(),
                self.loop_browser.track_volumes_db().to_vec(),
                Arc::clone(&self.play_state),
            ));
        }
    }

    pub(in crate::tui) fn finish_loop_browser(&mut self) {
        if let Some(mut playback) = self.loop_playback.take() {
            playback.stop();
        }
        let session = self.begin_playback_session();
        self.set_play_state_if_current(session, PlayState::Idle);
        self.mode = Mode::Normal;
    }

    pub(in crate::tui) fn preview_loop_file(&self, path: PathBuf) {
        if let Some(playback) = &self.loop_playback {
            playback.preview(path);
        }
    }

    pub(in crate::tui) fn trigger_loop_pad(&self, pad: char, path: PathBuf) {
        if let Some(playback) = &self.loop_playback {
            playback.trigger(pad, path);
        }
    }

    pub(in crate::tui) fn update_loop_grid(&self, grid: LoopPlaybackGrid) {
        if let Some(playback) = &self.loop_playback {
            playback.set_grid(grid);
        }
    }

    pub(in crate::tui) fn update_loop_track_volume(&self, track: usize, volume_db: i32) {
        if let Some(playback) = &self.loop_playback {
            playback.set_track_volume(track, volume_db);
        }
    }
}

fn playback_worker(
    receiver: mpsc::Receiver<LoopPlaybackCommand>,
    grid: LoopPlaybackGrid,
    mut track_volumes_db: Vec<i32>,
    state: &Arc<Mutex<PlayState>>,
) -> Result<()> {
    let (_stream, handle) =
        rodio::OutputStream::try_default().context("audio output deviceを開けません")?;
    let mut preview_sink: Option<Arc<rodio::Sink>> = None;
    let mut pad_sinks = HashMap::<char, Arc<rodio::Sink>>::new();
    let mut measure_sinks = Vec::<TrackSink>::new();
    let mut current_measure = None;
    let mut measure_deadline = None;
    let mut preparation = PreparationWorker::spawn();
    let mut pending_generation = Some(preparation.submit(grid));
    let mut active: Option<PreparedSet> = None;
    set_play_state(
        state,
        PlayState::Running(format!("BPM{TARGET_BPM:.0}変換中")),
    );

    loop {
        while let Some(result) = preparation.try_result() {
            if pending_generation != Some(result.generation) {
                continue;
            }
            pending_generation = None;
            active = Some(result.prepared);
            let prepared = active.as_ref().expect("just assigned");
            if let Some(warning) = &prepared.warning {
                set_play_state(state, PlayState::Err(warning.clone()));
            } else if prepared.grid.iter().flatten().any(Option::is_some) {
                set_play_state(
                    state,
                    PlayState::Playing(format!("BPM{TARGET_BPM:.0} 準備完了")),
                );
            } else {
                set_play_state(state, PlayState::Idle);
            }
        }
        pad_sinks.retain(|_, sink| !sink.empty());
        measure_sinks.retain(|voice| !voice.sink.empty());
        if measure_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            measure_deadline = None;
        }
        if measure_deadline.is_none() {
            if let Some(prepared) = active.as_ref() {
                if let Some(measure) = next_measure(&prepared.grid, current_measure) {
                    match start_measure(&handle, prepared, measure, &track_volumes_db) {
                        Ok(sinks) => {
                            current_measure = Some(measure);
                            measure_sinks.extend(sinks);
                            measure_deadline =
                                Some(Instant::now() + measure_duration(&prepared.grid));
                            if pending_generation.is_none() && prepared.warning.is_none() {
                                let profile = starting_clips(&prepared.grid, measure)
                                    .first()
                                    .map_or("-", |(_, clip)| profile_label(clip));
                                set_play_state(
                                    state,
                                    PlayState::Playing(format!(
                                        "BPM{TARGET_BPM:.0} loop measure {} {profile}",
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
                            current_measure = Some(measure);
                            measure_deadline =
                                Some(Instant::now() + measure_duration(&prepared.grid));
                        }
                    }
                } else {
                    current_measure = None;
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
            Ok(LoopPlaybackCommand::Preview(path)) => {
                if let Some(sink) = preview_sink.take() {
                    sink.stop();
                }
                match play_path(&handle, &path) {
                    Ok(sink) => preview_sink = Some(sink),
                    Err(error) => {
                        set_play_state(state, PlayState::Err(format!("WAV再生に失敗: {error}")))
                    }
                }
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
            Ok(LoopPlaybackCommand::SetGrid(next_grid)) => {
                pending_generation = Some(preparation.submit(next_grid));
                set_play_state(
                    state,
                    PlayState::Running(format!("BPM{TARGET_BPM:.0}変換中")),
                );
            }
            Ok(LoopPlaybackCommand::SetTrackVolume { track, volume_db }) => {
                if track >= track_volumes_db.len() {
                    track_volumes_db.resize(track + 1, 0);
                }
                track_volumes_db[track] = volume_db;
                let gain = crate::mixer_overlay::volume_db_to_gain(volume_db);
                for voice in measure_sinks.iter().filter(|voice| voice.track == track) {
                    voice.sink.set_volume(gain);
                }
            }
            Ok(LoopPlaybackCommand::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                preparation.cancel();
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

fn next_measure(grid: &LoopPlaybackGrid, current: Option<usize>) -> Option<usize> {
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

fn starting_clips(grid: &LoopPlaybackGrid, measure: usize) -> Vec<(usize, &LoopPlaybackClip)> {
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

fn grid_tempo_clip(grid: &LoopPlaybackGrid) -> Option<&LoopPlaybackClip> {
    let measures = grid.iter().map(Vec::len).max().unwrap_or(0);
    for measure in 0..measures {
        for track in grid {
            if let Some(clip) = track.get(measure).and_then(Option::as_ref) {
                return Some(clip);
            }
        }
    }
    None
}

fn measure_duration(grid: &LoopPlaybackGrid) -> Duration {
    let Some(clip) = grid_tempo_clip(grid) else {
        return Duration::from_millis(1);
    };
    let seconds = 60.0 / TARGET_BPM * f64::from(clip.meter_numerator) * 4.0
        / f64::from(clip.meter_denominator);
    if seconds.is_finite() && seconds > 0.0 {
        Duration::from_secs_f64(seconds).max(Duration::from_millis(1))
    } else {
        Duration::from_secs(2)
    }
}

fn start_measure(
    handle: &rodio::OutputStreamHandle,
    prepared: &PreparedSet,
    measure: usize,
    track_volumes_db: &[i32],
) -> Result<Vec<TrackSink>> {
    let clips = starting_clips(&prepared.grid, measure);
    let mut sinks = Vec::with_capacity(clips.len());
    for (track, clip) in clips {
        let Some(Ok(audio)) = prepared.audio_for(clip) else {
            continue;
        };
        let sink = Arc::new(rodio::Sink::try_new(handle)?);
        sink.set_volume(crate::mixer_overlay::volume_db_to_gain(
            track_volumes_db.get(track).copied().unwrap_or(0),
        ));
        sink.append(audio.source());
        sinks.push(TrackSink { track, sink });
    }
    Ok(sinks)
}

fn play_path(handle: &rodio::OutputStreamHandle, path: &Path) -> Result<Arc<rodio::Sink>> {
    let file = File::open(path).with_context(|| format!("WAVを開けません: {}", path.display()))?;
    let source = rodio::Decoder::new(BufReader::new(file))
        .with_context(|| format!("WAVをdecodeできません: {}", path.display()))?;
    let sink = Arc::new(rodio::Sink::try_new(handle)?);
    sink.append(source);
    Ok(sink)
}

fn stop_sinks(sinks: &mut Vec<TrackSink>) {
    for voice in sinks.drain(..) {
        voice.sink.stop();
    }
}

fn stop_pad_sinks(sinks: &mut HashMap<char, Arc<rodio::Sink>>) {
    for (_, sink) in sinks.drain() {
        sink.stop();
    }
}

fn take_pad_voice<T>(voices: &mut HashMap<char, T>, pad: char) -> Option<T> {
    voices.remove(&pad)
}

fn set_play_state(state: &Arc<Mutex<PlayState>>, next: PlayState) {
    *state.lock().unwrap() = next;
}

#[cfg(test)]
#[path = "playback/tests.rs"]
mod tests;
