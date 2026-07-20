use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use super::{LoopPlaybackClip, LoopPlaybackGrid};
use crate::tui::{Mode, PlayState, TuiApp};
use anyhow::{Context, Result};

enum LoopPlaybackCommand {
    Preview(PathBuf),
    Trigger { pad: char, path: PathBuf },
    SetGrid(LoopPlaybackGrid),
    Stop,
}

pub(in crate::tui) struct LoopPlaybackController {
    sender: mpsc::Sender<LoopPlaybackCommand>,
    worker: Option<JoinHandle<()>>,
}

impl LoopPlaybackController {
    fn spawn(grid: LoopPlaybackGrid, state: Arc<Mutex<PlayState>>) -> Self {
        let (sender, receiver) = mpsc::channel();
        let worker = std::thread::spawn(move || {
            if let Err(error) = playback_worker(receiver, grid, &state) {
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
}

fn playback_worker(
    receiver: mpsc::Receiver<LoopPlaybackCommand>,
    mut grid: LoopPlaybackGrid,
    state: &Arc<Mutex<PlayState>>,
) -> Result<()> {
    let (_stream, handle) =
        rodio::OutputStream::try_default().context("audio output deviceを開けません")?;
    let mut preview_sink: Option<Arc<rodio::Sink>> = None;
    let mut pad_sinks = HashMap::<char, Arc<rodio::Sink>>::new();
    let mut measure_sinks = Vec::<Arc<rodio::Sink>>::new();
    let mut current_measure = None;
    let mut measure_deadline = None;
    let mut playback_suspended = false;

    loop {
        pad_sinks.retain(|_, sink| !sink.empty());
        measure_sinks.retain(|sink| !sink.empty());
        if measure_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            measure_deadline = None;
        }
        if measure_deadline.is_none() && !playback_suspended {
            if let Some(measure) = next_measure(&grid, current_measure) {
                match start_measure(&handle, starting_paths(&grid, measure)) {
                    Ok(sinks) => {
                        current_measure = Some(measure);
                        measure_sinks.extend(sinks);
                        measure_deadline = Some(Instant::now() + measure_duration(&grid));
                        set_play_state(
                            state,
                            PlayState::Playing(format!("loop measure {}", measure + 1)),
                        );
                    }
                    Err(error) => {
                        set_play_state(
                            state,
                            PlayState::Err(format!("WAV loop再生に失敗: {error}")),
                        );
                        current_measure = Some(measure);
                        playback_suspended = true;
                    }
                }
            } else {
                current_measure = None;
                set_play_state(state, PlayState::Idle);
            }
        }

        let timeout = measure_deadline
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .unwrap_or(Duration::from_secs(60));
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
                grid = next_grid;
                playback_suspended = false;
            }
            Ok(LoopPlaybackCommand::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
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

fn starting_paths(grid: &LoopPlaybackGrid, measure: usize) -> Vec<PathBuf> {
    grid.iter()
        .filter_map(|track| {
            track
                .get(measure)
                .and_then(Option::as_ref)
                .map(|clip| clip.path.clone())
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
    let seconds =
        60.0 / clip.bpm * f64::from(clip.meter_numerator) * 4.0 / f64::from(clip.meter_denominator);
    if seconds.is_finite() && seconds > 0.0 {
        Duration::from_secs_f64(seconds).max(Duration::from_millis(1))
    } else {
        Duration::from_secs(2)
    }
}

fn start_measure(
    handle: &rodio::OutputStreamHandle,
    paths: Vec<PathBuf>,
) -> Result<Vec<Arc<rodio::Sink>>> {
    let mut sinks = Vec::with_capacity(paths.len());
    for path in paths {
        let file =
            File::open(&path).with_context(|| format!("WAVを開けません: {}", path.display()))?;
        let source = rodio::Decoder::new(BufReader::new(file))
            .with_context(|| format!("WAVをdecodeできません: {}", path.display()))?;
        let sink = Arc::new(rodio::Sink::try_new(handle)?);
        sink.append(source);
        sinks.push(sink);
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

fn stop_sinks(sinks: &mut Vec<Arc<rodio::Sink>>) {
    for sink in sinks.drain(..) {
        sink.stop();
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
mod tests {
    use super::*;

    fn clip(name: &str, span_measures: usize, bpm: f64) -> Option<LoopPlaybackClip> {
        Some(LoopPlaybackClip {
            path: PathBuf::from(name),
            span_measures,
            bpm,
            meter_numerator: 4,
            meter_denominator: 4,
        })
    }

    #[test]
    fn next_measure_skips_empty_columns_and_wraps() {
        let grid = vec![
            vec![clip("a.wav", 1, 120.0), None, clip("c.wav", 1, 120.0)],
            vec![clip("b.wav", 1, 90.0), None, None],
        ];
        assert_eq!(next_measure(&grid, None), Some(0));
        assert_eq!(next_measure(&grid, Some(0)), Some(2));
        assert_eq!(next_measure(&grid, Some(2)), Some(0));
        assert_eq!(
            starting_paths(&grid, 0),
            vec![PathBuf::from("a.wav"), PathBuf::from("b.wav")]
        );
    }

    #[test]
    fn next_measure_returns_none_for_empty_grid() {
        assert_eq!(next_measure(&vec![vec![None, None]], None), None);
    }

    #[test]
    fn tempo_uses_the_leftmost_then_topmost_clip() {
        let grid = vec![
            vec![None, clip("top.wav", 1, 120.0)],
            vec![clip("first.wav", 2, 100.0), None],
        ];
        assert_eq!(measure_duration(&grid), Duration::from_millis(2_400));
    }

    #[test]
    fn continuation_measure_waits_and_can_start_another_track() {
        let grid = vec![
            vec![clip("long.wav", 2, 120.0), None],
            vec![None, clip("next.wav", 1, 120.0)],
        ];
        assert_eq!(next_measure(&grid, None), Some(0));
        assert_eq!(next_measure(&grid, Some(0)), Some(1));
        assert_eq!(starting_paths(&grid, 1), vec![PathBuf::from("next.wav")]);
    }

    #[test]
    fn updated_grid_is_used_when_the_next_measure_is_selected() {
        let before = vec![vec![
            clip("current.wav", 1, 120.0),
            None,
            clip("old.wav", 1, 120.0),
        ]];
        assert_eq!(next_measure(&before, Some(0)), Some(2));

        let after = vec![vec![
            clip("current.wav", 1, 120.0),
            clip("new.wav", 1, 120.0),
            None,
        ]];
        assert_eq!(next_measure(&after, Some(0)), Some(1));
    }

    #[test]
    fn pad_voices_replace_only_the_same_pad() {
        let mut voices = HashMap::from([('c', "first-c"), ('d', "first-d")]);

        assert_eq!(take_pad_voice(&mut voices, 'c'), Some("first-c"));
        voices.insert('c', "second-c");
        assert_eq!(voices.get(&'c'), Some(&"second-c"));
        assert_eq!(voices.get(&'d'), Some(&"first-d"));
    }
}
