use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use super::{PlayState, TuiApp};

impl<'a> TuiApp<'a> {
    pub(super) fn playback_session_is_current(
        playback_session: &std::sync::atomic::AtomicU64,
        session: u64,
    ) -> bool {
        playback_session.load(Ordering::Acquire) == session
    }

    pub(super) fn set_play_state_for_session(
        state: &Mutex<PlayState>,
        playback_session: &std::sync::atomic::AtomicU64,
        session: u64,
        next_state: PlayState,
    ) {
        let mut state = state.lock().unwrap();
        if Self::playback_session_is_current(playback_session, session) {
            *state = next_state;
        }
    }

    pub(super) fn clear_active_sink_for_session(
        active_sink: &Mutex<Option<Arc<rodio::Sink>>>,
        playback_session: &std::sync::atomic::AtomicU64,
        session: u64,
    ) {
        let mut active_sink = active_sink.lock().unwrap();
        if Self::playback_session_is_current(playback_session, session) {
            active_sink.take();
        }
    }

    pub(super) fn play_samples_for_session(
        state: &Mutex<PlayState>,
        playback_session: &std::sync::atomic::AtomicU64,
        active_sink: &Mutex<Option<Arc<rodio::Sink>>>,
        session: u64,
        sample_rate: u32,
        samples: Vec<f32>,
        msg: String,
    ) {
        if !Self::playback_session_is_current(playback_session, session) {
            return;
        }

        let (_stream, stream_handle) = match rodio::OutputStream::try_default() {
            Ok(stream_and_handle) => stream_and_handle,
            Err(e) => {
                Self::clear_active_sink_for_session(active_sink, playback_session, session);
                Self::set_play_state_for_session(
                    state,
                    playback_session,
                    session,
                    PlayState::Err(format!("エラー: audio init failed: {e}")),
                );
                return;
            }
        };
        let sink = match rodio::Sink::try_new(&stream_handle) {
            Ok(sink) => sink,
            Err(e) => {
                Self::clear_active_sink_for_session(active_sink, playback_session, session);
                Self::set_play_state_for_session(
                    state,
                    playback_session,
                    session,
                    PlayState::Err(format!("エラー: sink init failed: {e}")),
                );
                return;
            }
        };
        let sink = Arc::new(sink);
        sink.append(rodio::buffer::SamplesBuffer::new(2, sample_rate, samples));
        {
            let mut active_sink_guard = active_sink.lock().unwrap();
            if !Self::playback_session_is_current(playback_session, session) {
                sink.stop();
                return;
            }
            *active_sink_guard = Some(Arc::clone(&sink));
        }
        sink.sleep_until_end();

        Self::clear_active_sink_for_session(active_sink, playback_session, session);
        Self::set_play_state_for_session(state, playback_session, session, PlayState::Done(msg));
    }

    pub(super) fn begin_playback_session(&self) -> u64 {
        let session = self.playback_session.fetch_add(1, Ordering::AcqRel) + 1;
        if let Some(sink) = self.active_sink.lock().unwrap().take() {
            sink.stop();
        }
        if let Some(play_server) = &self.realtime_play_server {
            let _ = play_server.stop();
        }
        session
    }

    pub(super) fn set_play_state_if_current(&self, session: u64, next_state: PlayState) {
        Self::set_play_state_for_session(
            &self.play_state,
            &self.playback_session,
            session,
            next_state,
        );
    }
}
