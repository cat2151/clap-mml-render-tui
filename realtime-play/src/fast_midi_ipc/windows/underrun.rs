use std::sync::{atomic::Ordering, Arc};

use super::Mapping;

/// Reads the counters the server publishes (output underruns, dropped queued events)
/// without waiting for command IPC.
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

    /// サーバーが待ち行列満杯で捨てた MIDI イベントの累計。
    pub fn dropped_live_events_total(&self) -> u64 {
        self.mapping
            .ring()
            .dropped_live_events_total
            .load(Ordering::Acquire)
    }
}
