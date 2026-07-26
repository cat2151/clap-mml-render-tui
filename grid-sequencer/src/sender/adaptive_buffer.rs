use std::time::{Duration, Instant};

pub(super) const INITIAL_BUFFER_MULTIPLIER: u8 = 2;
pub(super) const RESTORE_BUFFER_MULTIPLIER: u8 = 4;
const STABLE_INTERVAL: Duration = Duration::from_secs(10);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct AdaptiveBuffer {
    multiplier: u8,
    underrun_frames: u64,
    last_total_underrun_frames: u64,
    stable_since: Instant,
}

impl AdaptiveBuffer {
    pub(super) fn new(now: Instant, total_underrun_frames: u64) -> Self {
        Self {
            multiplier: INITIAL_BUFFER_MULTIPLIER,
            underrun_frames: 0,
            last_total_underrun_frames: total_underrun_frames,
            stable_since: now,
        }
    }

    pub(super) fn multiplier(self) -> u8 {
        self.multiplier
    }

    pub(super) fn underrun_frames(self) -> u64 {
        self.underrun_frames
    }

    /// Returns a new multiplier when the server setting must change.
    pub(super) fn observe(&mut self, now: Instant, total_underrun_frames: u64) -> Option<u8> {
        if total_underrun_frames < self.last_total_underrun_frames {
            self.last_total_underrun_frames = total_underrun_frames;
            self.underrun_frames = 0;
            self.stable_since = now;
            return None;
        }

        let new_underruns = total_underrun_frames.saturating_sub(self.last_total_underrun_frames);
        self.last_total_underrun_frames = total_underrun_frames;
        if new_underruns > 0 {
            self.underrun_frames = self.underrun_frames.saturating_add(new_underruns);
            self.stable_since = now;
            if let Some(next) = next_larger(self.multiplier) {
                self.multiplier = next;
                self.underrun_frames = 0;
                return Some(next);
            }
            return None;
        }

        if now.saturating_duration_since(self.stable_since) >= STABLE_INTERVAL {
            if let Some(next) = next_smaller(self.multiplier) {
                self.multiplier = next;
                self.underrun_frames = 0;
                self.stable_since = now;
                return Some(next);
            }
        }
        None
    }
}

fn next_larger(multiplier: u8) -> Option<u8> {
    match multiplier {
        2 => Some(4),
        4 => Some(8),
        8 => Some(16),
        _ => None,
    }
}

fn next_smaller(multiplier: u8) -> Option<u8> {
    match multiplier {
        16 => Some(8),
        8 => Some(4),
        4 => Some(2),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
