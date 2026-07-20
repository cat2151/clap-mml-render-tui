use std::fs::File;
use std::io::BufReader;
use std::path::{Path, PathBuf};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use rodio::Source;

use super::LoopPlaybackGrid;
use crate::tui::{Mode, PlayState, TuiApp};

enum LoopPlaybackCommand {
    Preview(PathBuf),
    Trigger(PathBuf),
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

    pub(super) fn trigger(&self, path: PathBuf) {
        let _ = self.sender.send(LoopPlaybackCommand::Trigger(path));
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

    pub(in crate::tui) fn trigger_loop_pad(&self, path: PathBuf) {
        if let Some(playback) = &self.loop_playback {
            playback.trigger(path);
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
    let mut one_shot_sinks = Vec::<Arc<rodio::Sink>>::new();
    let mut measure_sinks = Vec::<Arc<rodio::Sink>>::new();
    let mut current_measure = None;
    let mut measure_deadline = None;
    let mut playback_suspended = false;

    loop {
        one_shot_sinks.retain(|sink| !sink.empty());
        if measure_deadline.is_some_and(|deadline| Instant::now() >= deadline) {
            measure_sinks.clear();
            measure_deadline = None;
        }
        if measure_deadline.is_none() && !playback_suspended {
            if let Some((measure, paths)) = next_measure(&grid, current_measure) {
                match start_measure(&handle, &paths) {
                    Ok((sinks, duration)) => {
                        current_measure = Some(measure);
                        measure_sinks = sinks;
                        measure_deadline = Some(Instant::now() + duration);
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
            Ok(LoopPlaybackCommand::Trigger(path)) => match play_path(&handle, &path) {
                Ok(sink) => one_shot_sinks.push(sink),
                Err(error) => {
                    set_play_state(state, PlayState::Err(format!("WAV pad再生に失敗: {error}")))
                }
            },
            Ok(LoopPlaybackCommand::SetGrid(next_grid)) => {
                grid = next_grid;
                playback_suspended = false;
            }
            Ok(LoopPlaybackCommand::Stop) | Err(mpsc::RecvTimeoutError::Disconnected) => {
                stop_sinks(&mut measure_sinks);
                stop_sinks(&mut one_shot_sinks);
                if let Some(sink) = preview_sink.take() {
                    sink.stop();
                }
                return Ok(());
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
        }
    }
}

fn next_measure(grid: &LoopPlaybackGrid, current: Option<usize>) -> Option<(usize, Vec<PathBuf>)> {
    let measures = grid.iter().map(Vec::len).max().unwrap_or(0);
    if measures == 0 {
        return None;
    }
    let start = current.map_or(0, |measure| (measure + 1) % measures);
    for offset in 0..measures {
        let measure = (start + offset) % measures;
        let paths = grid
            .iter()
            .filter_map(|track| track.get(measure).and_then(Option::as_ref).cloned())
            .collect::<Vec<_>>();
        if !paths.is_empty() {
            return Some((measure, paths));
        }
    }
    None
}

fn start_measure(
    handle: &rodio::OutputStreamHandle,
    paths: &[PathBuf],
) -> Result<(Vec<Arc<rodio::Sink>>, Duration)> {
    let mut sinks = Vec::with_capacity(paths.len());
    let mut source_durations = Vec::with_capacity(paths.len());
    for path in paths {
        let file =
            File::open(path).with_context(|| format!("WAVを開けません: {}", path.display()))?;
        let source = rodio::Decoder::new(BufReader::new(file))
            .with_context(|| format!("WAVをdecodeできません: {}", path.display()))?;
        let source_duration = source
            .total_duration()
            .ok_or_else(|| anyhow::anyhow!("WAVの長さを取得できません: {}", path.display()))?;
        let sink = Arc::new(rodio::Sink::try_new(handle)?);
        sink.append(source);
        sinks.push(sink);
        source_durations.push(source_duration);
    }
    Ok((sinks, longest_duration(&source_durations)))
}

fn longest_duration(durations: &[Duration]) -> Duration {
    durations
        .iter()
        .copied()
        .max()
        .unwrap_or(Duration::ZERO)
        .max(Duration::from_millis(1))
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

fn set_play_state(state: &Arc<Mutex<PlayState>>, next: PlayState) {
    *state.lock().unwrap() = next;
}

#[cfg(test)]
mod tests {
    use super::*;

    fn path(name: &str) -> Option<PathBuf> {
        Some(PathBuf::from(name))
    }

    #[test]
    fn next_measure_skips_empty_columns_and_wraps() {
        let grid = vec![
            vec![path("a.wav"), None, path("c.wav")],
            vec![path("b.wav"), None, None],
        ];
        assert_eq!(
            next_measure(&grid, None),
            Some((0, vec![PathBuf::from("a.wav"), PathBuf::from("b.wav")]))
        );
        assert_eq!(
            next_measure(&grid, Some(0)),
            Some((2, vec![PathBuf::from("c.wav")]))
        );
        assert_eq!(next_measure(&grid, Some(2)).unwrap().0, 0);
    }

    #[test]
    fn next_measure_returns_none_for_empty_grid() {
        assert_eq!(next_measure(&vec![vec![None, None]], None), None);
    }

    #[test]
    fn measure_duration_uses_the_longest_wav() {
        assert_eq!(
            longest_duration(&[
                Duration::from_millis(250),
                Duration::from_millis(900),
                Duration::from_millis(500),
            ]),
            Duration::from_millis(900)
        );
    }

    #[test]
    fn updated_grid_is_used_when_the_next_measure_is_selected() {
        let before = vec![vec![path("current.wav"), None, path("old.wav")]];
        assert_eq!(next_measure(&before, Some(0)).unwrap().0, 2);

        let after = vec![vec![path("current.wav"), path("new.wav"), None]];
        assert_eq!(
            next_measure(&after, Some(0)),
            Some((1, vec![PathBuf::from("new.wav")]))
        );
    }
}
