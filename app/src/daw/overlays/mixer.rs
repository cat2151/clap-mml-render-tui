pub(in crate::daw) struct DawMixerOverlayState {
    pub(in crate::daw) cursor_track: usize,
}

impl DawMixerOverlayState {
    pub(in crate::daw) fn new(cursor_track: usize) -> Self {
        Self { cursor_track }
    }
}
