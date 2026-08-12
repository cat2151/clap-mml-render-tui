use ratatui::Frame;

use crate::LoopBrowser;

pub(super) fn draw_overlay(frame: &mut Frame<'_>, state: &LoopBrowser) {
    let Some(input) = state.bpm_input.as_ref() else {
        return;
    };
    cmrt_tui_core::bpm::overlay::draw(
        frame,
        input,
        state.target_bpm().bpm,
        state.bpm_mode(),
        state.bpm_range(),
    );
}
