pub(crate) struct DawMixerOverlayState {
    pub(crate) cursor_track: usize,
}

impl DawMixerOverlayState {
    pub(crate) fn new(cursor_track: usize) -> Self {
        Self { cursor_track }
    }
}
