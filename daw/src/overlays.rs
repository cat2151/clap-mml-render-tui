mod history;
mod mixer;
mod patch_select;
mod project;

pub(crate) use history::DawHistoryOverlayState;
pub(crate) use mixer::DawMixerOverlayState;
pub(crate) use patch_select::DawPatchSelectOverlayState;
pub(crate) use project::DawProjectOverlayState;

pub(crate) struct DawOverlays {
    pub(crate) mixer: DawMixerOverlayState,
    pub(crate) history: DawHistoryOverlayState,
    pub(crate) patch_select: DawPatchSelectOverlayState,
    pub(crate) project: DawProjectOverlayState,
    pub(crate) screen_switch: cmrt_tui_core::screen_switch::ScreenSwitchMenu,
}

impl DawOverlays {
    pub(crate) fn new(mixer_cursor_track: usize) -> Self {
        Self {
            mixer: DawMixerOverlayState::new(mixer_cursor_track),
            history: DawHistoryOverlayState::new(),
            patch_select: DawPatchSelectOverlayState::new(),
            project: DawProjectOverlayState::new(),
            screen_switch: cmrt_tui_core::screen_switch::ScreenSwitchMenu::default(),
        }
    }
}
