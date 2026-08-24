use std::sync::{atomic::Ordering, Arc};

use super::Mapping;

/// Reads the audio callback's underrun counter without waiting for command IPC.
#[derive(Clone)]
pub struct FastMidiUnderrunReader {
    mapping: Arc<Mapping>,
}

impl FastMidiUnderrunReader {
    pub(super) fn new(mapping: Arc<Mapping>) -> Self {
        Self { mapping }
    }

    pub fn underrun_frames(&self) -> u64 {
        self.mapping.ring().underrun_frames.load(Ordering::Acquire)
    }
}
