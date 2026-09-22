use std::sync::Arc;

use crate::DawApp;

impl DawApp {
    pub(super) fn start_preview_with_snapshot_via_play_server(
        &self,
        measure_index: usize,
        track_mmls: Vec<String>,
        active_tracks: Vec<usize>,
        measure_samples: usize,
    ) {
        let Some(play_server) = self.playback.realtime_play_server.as_ref().cloned() else {
            self.append_log_line("preview: realtime play server is not initialized");
            return;
        };

        let preview_output = self.playback.preview_output.clone();
        let log_lines = Arc::clone(&self.log_lines);
        let sample_rate = self.cfg.sample_rate as u32;

        let session = preview_output.start_session_with(|| {
            let _ = play_server.stop();
        });
        crate::append_log_line(&log_lines, format!("preview: meas{}", measure_index + 1));

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
                crate::append_log_line(
                    &log_lines,
                    format!("meas{}: play-server error: {}", measure_index + 1, error),
                );
            } else {
                let measure_duration = std::time::Duration::from_secs_f64(
                    measure_samples as f64 / (sample_rate as f64 * 2.0),
                );
                let preview_active = preview_output.enqueue_if_current(
                    session,
                    measure_index,
                    measure_duration,
                    None,
                    || {},
                );
                if preview_active {
                    preview_output.wait_until_end(session, measure_duration);
                }
            }

            if preview_output.finish_session_with(session, || {
                let _ = play_server.stop();
            }) {
                crate::append_log_line(&log_lines, "preview: finished");
            }
        });
    }
}
