mod history;
mod mixer;
mod patch_select;

pub(in crate::daw) use history::DawHistoryOverlayState;
pub(in crate::daw) use mixer::DawMixerOverlayState;
pub(in crate::daw) use patch_select::DawPatchSelectOverlayState;

pub(in crate::daw) struct DawOverlays {
    pub(in crate::daw) mixer: DawMixerOverlayState,
    pub(in crate::daw) history: DawHistoryOverlayState,
    pub(in crate::daw) patch_select: DawPatchSelectOverlayState,
}

impl DawOverlays {
    pub(in crate::daw) fn new(mixer_cursor_track: usize) -> Self {
        Self {
            mixer: DawMixerOverlayState::new(mixer_cursor_track),
            history: DawHistoryOverlayState::new(),
            patch_select: DawPatchSelectOverlayState::new(),
        }
    }
}
