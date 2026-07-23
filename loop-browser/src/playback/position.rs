use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub type SharedPlaybackPosition = Arc<Mutex<Option<ScheduledMeasure>>>;

#[derive(Clone, Copy, Debug)]
pub struct ScheduledMeasure {
    measure: usize,
    started_at: Instant,
    duration: Duration,
    beats_per_measure: usize,
}

impl ScheduledMeasure {
    pub fn new(
        measure: usize,
        started_at: Instant,
        duration: Duration,
        beats_per_measure: usize,
    ) -> Self {
        Self {
            measure,
            started_at,
            duration,
            beats_per_measure,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PlaybackBeat {
    pub measure: usize,
    pub beat: usize,
    pub beats_per_measure: usize,
}

pub fn new_shared() -> SharedPlaybackPosition {
    Arc::new(Mutex::new(None))
}

pub fn set(shared: &SharedPlaybackPosition, position: ScheduledMeasure) {
    *shared.lock().unwrap() = Some(position);
}

pub fn clear(shared: &SharedPlaybackPosition) {
    *shared.lock().unwrap() = None;
}

pub fn current_beat(shared: &SharedPlaybackPosition, now: Instant) -> Option<PlaybackBeat> {
    let position = *shared.lock().unwrap();
    position.and_then(|position| position.current_beat(now))
}

#[cfg(any(test, feature = "test-support"))]
pub fn set_beat_for_test(
    shared: &SharedPlaybackPosition,
    measure: usize,
    beat: usize,
    beats_per_measure: usize,
) {
    let beats_per_measure = beats_per_measure.max(1);
    let beat = beat.min(beats_per_measure - 1);
    let duration = Duration::from_secs(3_600);
    let elapsed = duration.mul_f64((beat as f64 + 0.25) / beats_per_measure as f64);
    set(
        shared,
        ScheduledMeasure::new(
            measure,
            Instant::now() - elapsed,
            duration,
            beats_per_measure,
        ),
    );
}

impl ScheduledMeasure {
    fn current_beat(self, now: Instant) -> Option<PlaybackBeat> {
        if self.beats_per_measure == 0 || self.duration.is_zero() || now < self.started_at {
            return None;
        }
        let elapsed = now.duration_since(self.started_at);
        if elapsed >= self.duration {
            return None;
        }
        let beat = ((elapsed.as_secs_f64() / self.duration.as_secs_f64())
            * self.beats_per_measure as f64)
            .floor() as usize;
        Some(PlaybackBeat {
            measure: self.measure,
            beat: beat.min(self.beats_per_measure - 1),
            beats_per_measure: self.beats_per_measure,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn beat_advances_at_each_quarter_of_a_four_beat_measure() {
        let start = Instant::now();
        let position = ScheduledMeasure::new(2, start, Duration::from_secs(2), 4);

        assert_eq!(
            position.current_beat(start),
            Some(PlaybackBeat {
                measure: 2,
                beat: 0,
                beats_per_measure: 4,
            })
        );
        assert_eq!(
            position.current_beat(start + Duration::from_millis(500)),
            Some(PlaybackBeat {
                measure: 2,
                beat: 1,
                beats_per_measure: 4,
            })
        );
        assert_eq!(
            position.current_beat(start + Duration::from_millis(1_999)),
            Some(PlaybackBeat {
                measure: 2,
                beat: 3,
                beats_per_measure: 4,
            })
        );
    }

    #[test]
    fn marker_is_hidden_before_start_and_at_the_measure_deadline() {
        let start = Instant::now() + Duration::from_secs(1);
        let position = ScheduledMeasure::new(0, start, Duration::from_secs(2), 3);

        assert_eq!(
            position.current_beat(start - Duration::from_millis(1)),
            None
        );
        assert_eq!(position.current_beat(start + Duration::from_secs(2)), None);
    }
}
