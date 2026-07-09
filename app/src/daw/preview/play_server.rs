use std::sync::atomic::Ordering;
use std::sync::Arc;

use super::{
    begin_preview_output, wait_preview_duration, PreviewOutputRequest, PreviewOutputState,
};
use crate::daw::{DawApp, DawPlayState};

impl DawApp {
    pub(super) fn start_preview_with_snapshot_via_play_server(
        &self,
        measure_index: usize,
        track_mmls: Vec<String>,
        active_tracks: Vec<usize>,
    ) {
        let Some(play_server) = self.realtime_play_server.as_ref().cloned() else {
            self.append_log_line("preview: realtime play server is not initialized");
            return;
        };

        let measure_samples = self.measure_duration_samples();
        let play_state = Arc::clone(&self.play_state);
        let play_transition_lock = Arc::clone(&self.play_transition_lock);
        let preview_session = Arc::clone(&self.preview_session);
        let preview_sink = Arc::clone(&self.preview_sink);
        let play_position = Arc::clone(&self.play_position);
        let log_lines = Arc::clone(&self.log_lines);
        let sample_rate = self.cfg.sample_rate as u32;

        let session = {
            let _transition_guard = play_transition_lock.lock().unwrap();
            if let Some(sink) = preview_sink.lock().unwrap().take() {
                sink.stop();
            }
            let _ = play_server.stop();
            *play_position.lock().unwrap() = None;
            let session = preview_session.fetch_add(1, Ordering::AcqRel) + 1;
            *play_state.lock().unwrap() = DawPlayState::Preview;
            session
        };
        crate::logging::append_log_line(&log_lines, format!("preview: meas{}", measure_index + 1));

        std::thread::spawn(move || {
            let mml = active_tracks
                .iter()
                .filter_map(|track| track_mmls.get(*track))
                .map(String::as_str)
                .filter(|mml| !mml.trim().is_empty())
                .collect::<Vec<_>>()
                .join(";");
            let result =
                cmrt_core::mml_to_smf_bytes(&mml).and_then(|smf| play_server.play_smf(smf));
            if let Err(error) = result {
                crate::logging::append_log_line(
                    &log_lines,
                    format!("meas{}: play-server error: {}", measure_index + 1, error),
                );
            } else {
                let measure_duration = std::time::Duration::from_secs_f64(
                    measure_samples as f64 / (sample_rate as f64 * 2.0),
                );
                let preview_active = begin_preview_output(
                    PreviewOutputState {
                        play_transition_lock: &play_transition_lock,
                        play_state: &play_state,
                        play_position: &play_position,
                        preview_session: &preview_session,
                    },
                    PreviewOutputRequest {
                        session,
                        measure_index,
                        measure_duration,
                    },
                    || {},
                );
                if preview_active {
                    wait_preview_duration(&play_state, &preview_session, session, measure_duration);
                }
            }

            let mut state = play_state.lock().unwrap();
            if *state == DawPlayState::Preview && preview_session.load(Ordering::Acquire) == session
            {
                let _ = play_server.stop();
                *state = DawPlayState::Idle;
                drop(state);
                *play_position.lock().unwrap() = None;
                crate::logging::append_log_line(&log_lines, "preview: finished");
            }
        });
    }
}
