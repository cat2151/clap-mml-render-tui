use std::{sync::Arc, time::Instant};

use super::{
    current_play_measure_index, effective_measure_count, following_measure_index,
    format_playback_measure_advance_log, format_playback_measure_resolution_log, measure_duration,
    wait_until_or_stop, DawApp, DawPlayState, PlayPosition,
};

impl DawApp {
    pub(super) fn start_play_from_measure_via_play_server(&self, start_measure_index: usize) {
        let Some(play_server) = self.realtime_play_server.as_ref().cloned() else {
            self.append_log_line("play: realtime play server is not initialized");
            *self.play_state.lock().unwrap() = DawPlayState::Idle;
            return;
        };

        let play_state = Arc::clone(&self.play_state);
        let play_position = Arc::clone(&self.play_position);
        let ab_repeat = Arc::clone(&self.ab_repeat);
        let play_measure_mmls = Arc::clone(&self.play_measure_mmls);
        let play_measure_samples = Arc::clone(&self.play_measure_samples);
        let log_lines = Arc::clone(&self.log_lines);
        let sample_rate = self.cfg.sample_rate as u32;

        std::thread::spawn(move || {
            let mut measure_index = start_measure_index;

            'outer: loop {
                if *play_state.lock().unwrap() != DawPlayState::Playing {
                    break;
                }

                let mmls = play_measure_mmls.lock().unwrap().clone();
                let measure_samples = *play_measure_samples.lock().unwrap();
                let effective_count = match effective_measure_count(&mmls) {
                    Some(n) => n,
                    None => break 'outer,
                };
                let ab_repeat_range =
                    (*ab_repeat.lock().unwrap()).normalized_range(effective_count);
                let current_measure_index =
                    current_play_measure_index(measure_index, effective_count, ab_repeat_range);
                let measure_duration = measure_duration(measure_samples, sample_rate);
                let measure_start = Instant::now();
                *play_position.lock().unwrap() = Some(PlayPosition {
                    measure_index: current_measure_index,
                    measure_start,
                    measure_duration,
                });
                crate::logging::append_log_line(
                    &log_lines,
                    format_playback_measure_resolution_log(
                        measure_index,
                        current_measure_index,
                        effective_count,
                    ),
                );

                let mml = mmls
                    .get(current_measure_index)
                    .map(String::as_str)
                    .unwrap_or_default();
                if mml.trim().is_empty() {
                    let _ = play_server.stop();
                    crate::logging::append_log_line(
                        &log_lines,
                        format!("meas{}: play-server empty", current_measure_index + 1),
                    );
                } else {
                    match cmrt_core::mml_to_smf_bytes(mml).and_then(|smf| play_server.play_smf(smf))
                    {
                        Ok(()) => crate::logging::append_log_line(
                            &log_lines,
                            format!("meas{}: play-server", current_measure_index + 1),
                        ),
                        Err(_) => {
                            crate::logging::append_log_line(
                                &log_lines,
                                format!("meas{}: play-server error", current_measure_index + 1),
                            );
                            break 'outer;
                        }
                    }
                }

                let next_measure_start = measure_start + measure_duration;
                if !wait_until_or_stop(&play_state, next_measure_start) {
                    break 'outer;
                }

                let next_mmls = play_measure_mmls.lock().unwrap().clone();
                let next_effective_count = match effective_measure_count(&next_mmls) {
                    Some(n) => n,
                    None => break 'outer,
                };
                let next_ab_repeat_range =
                    (*ab_repeat.lock().unwrap()).normalized_range(next_effective_count);
                let next_measure_index = following_measure_index(
                    current_measure_index,
                    next_effective_count,
                    next_ab_repeat_range,
                );
                crate::logging::append_log_line(
                    &log_lines,
                    format_playback_measure_advance_log(
                        current_measure_index,
                        next_measure_index,
                        next_effective_count,
                    ),
                );
                measure_index = next_measure_index;
            }

            let _ = play_server.stop();
            let mut state = play_state.lock().unwrap();
            if *state == DawPlayState::Playing {
                *state = DawPlayState::Idle;
                drop(state);
                *play_position.lock().unwrap() = None;
                crate::logging::append_log_line(&log_lines, "play: finished");
            }
        });
    }
}
