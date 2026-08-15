use std::sync::{Arc, Mutex};

use cmrt_loop_browser_domain::time_stretch::format_bpm;
use cmrt_tui_core::PlayState;

pub(super) fn set_preparation_state(state: &Arc<Mutex<PlayState>>, paused: bool, bpm: f64) {
    if paused {
        set_play_state(state, PlayState::Idle);
    } else {
        set_play_state(
            state,
            PlayState::Running(format!("BPM{}変換中", format_bpm(bpm))),
        );
    }
}

pub(super) fn set_play_state(state: &Arc<Mutex<PlayState>>, next: PlayState) {
    *state.lock().unwrap() = next;
}
