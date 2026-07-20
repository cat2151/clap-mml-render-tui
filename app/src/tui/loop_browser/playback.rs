use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::tui::{Mode, PlayState, TuiApp};

impl<'a> TuiApp<'a> {
    pub(in crate::tui) fn start_loop_browser(&mut self) {
        let session = self.begin_playback_session();
        self.set_play_state_if_current(session, PlayState::Idle);
        self.loop_browser.reload(&self.cfg);
        self.mode = Mode::LoopBrowser;
    }

    pub(in crate::tui) fn finish_loop_browser(&mut self) {
        let session = self.begin_playback_session();
        self.set_play_state_if_current(session, PlayState::Idle);
        self.mode = Mode::Normal;
    }

    pub(in crate::tui) fn play_loop_file(&self, path: PathBuf) {
        let session = self.begin_playback_session();
        let display = path.to_string_lossy().into_owned();
        self.set_play_state_if_current(session, PlayState::Playing(display.clone()));
        let state = Arc::clone(&self.play_state);
        let playback_session = Arc::clone(&self.playback_session);
        let active_sink = Arc::clone(&self.active_sink);
        std::thread::spawn(move || {
            let result = play_file_for_session(&path, session, &playback_session, &active_sink);
            if let Err(error) = result {
                TuiApp::clear_active_sink_for_session(&active_sink, &playback_session, session);
                TuiApp::set_play_state_for_session(
                    &state,
                    &playback_session,
                    session,
                    PlayState::Err(format!("WAV再生に失敗: {error}")),
                );
            } else {
                TuiApp::clear_active_sink_for_session(&active_sink, &playback_session, session);
                TuiApp::set_play_state_for_session(
                    &state,
                    &playback_session,
                    session,
                    PlayState::Done(display),
                );
            }
        });
    }
}

fn play_file_for_session(
    path: &Path,
    session: u64,
    playback_session: &std::sync::atomic::AtomicU64,
    active_sink: &std::sync::Mutex<Option<Arc<rodio::Sink>>>,
) -> anyhow::Result<()> {
    let file = std::fs::File::open(path)?;
    let source = rodio::Decoder::new(std::io::BufReader::new(file))?;
    let (_stream, stream_handle) = rodio::OutputStream::try_default()?;
    let sink = Arc::new(rodio::Sink::try_new(&stream_handle)?);
    if !TuiApp::playback_session_is_current(playback_session, session) {
        return Ok(());
    }
    {
        let mut guard = active_sink.lock().unwrap();
        if !TuiApp::playback_session_is_current(playback_session, session) {
            return Ok(());
        }
        *guard = Some(Arc::clone(&sink));
    }
    sink.append(source);
    sink.sleep_until_end();
    Ok(())
}
