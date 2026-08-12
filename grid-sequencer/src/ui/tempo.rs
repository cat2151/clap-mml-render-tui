use ratatui::Frame;

use crate::GridSequencerScreen;

pub(super) fn draw_overlay(frame: &mut Frame<'_>, screen: &GridSequencerScreen) {
    let Some(input) = screen.bpm_input.as_ref() else {
        return;
    };
    cmrt_tui_core::bpm::overlay::draw(
        frame,
        input,
        screen.bpm(),
        screen.bpm_mode(),
        screen.bpm_range(),
    );
}
