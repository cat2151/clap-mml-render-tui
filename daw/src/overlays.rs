mod history;
mod mixer;
mod patch_select;

pub(crate) use history::DawHistoryOverlayState;
pub(crate) use mixer::DawMixerOverlayState;
pub(crate) use patch_select::DawPatchSelectOverlayState;

pub(crate) struct DawOverlays {
    pub(crate) mixer: DawMixerOverlayState,
    pub(crate) history: DawHistoryOverlayState,
    pub(crate) patch_select: DawPatchSelectOverlayState,
    pub(crate) screen_switch: cmrt_tui_core::screen_switch::ScreenSwitchMenu,
}

impl DawOverlays {
    pub(crate) fn new(mixer_cursor_track: usize) -> Self {
        Self {
            mixer: DawMixerOverlayState::new(mixer_cursor_track),
            history: DawHistoryOverlayState::new(),
            patch_select: DawPatchSelectOverlayState::new(),
            screen_switch: cmrt_tui_core::screen_switch::ScreenSwitchMenu::default(),
        }
    }
}
