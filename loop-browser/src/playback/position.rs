use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub type SharedPlaybackPosition = Arc<Mutex<Option<ScheduledMeasure>>>;

#[derive(Clone, Copy, Debug)]
pub struct ScheduledMeasure {
    measure: usize,
    started_at: Instant,
    duration: Duration,
    beats_per_measure: usize,
    /// 鳴っているグリッドを何周し終えたか。
    cycle: u64,
    /// 鳴っているグリッドの識別子。UI が先読みを予約したときに採番した値が返ってくる。
    grid_token: u64,
}

impl ScheduledMeasure {
    pub fn new(
        measure: usize,
        started_at: Instant,
        duration: Duration,
        beats_per_measure: usize,
        cycle: u64,
        grid_token: u64,
    ) -> Self {
        Self {
            measure,
            started_at,
            duration,
            beats_per_measure,
            cycle,
            grid_token,
        }
    }
}

/// いま鳴っているグリッドの `(周回数, token)`。停止中・準備中は `None`。
pub fn current_grid(shared: &SharedPlaybackPosition) -> Option<(u64, u64)> {
    shared
        .lock()
        .unwrap()
        .map(|position| (position.cycle, position.grid_token))
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
    // 小節の何割まで進んだ位置か。`beat` の頭ちょうどに置くと丸めで前の beat へ倒れうる
    // ので、beat の 1/4 だけ内側へ寄せる。
    let ratio = (beat as f64 + 0.25) / beats_per_measure as f64;
    // OS 起動直後は `Instant` の原点が近く、1時間ぶん遡ると overflow する（実際に踏んだ）。
    // 遡れるところまで小節長を縮めても経過位置の比率は同じなので、報告される beat は
    // 変わらない。
    let mut duration = Duration::from_secs(3_600);
    let started_at = loop {
        if let Some(started_at) = Instant::now().checked_sub(duration.mul_f64(ratio)) {
            break started_at;
        }
        duration /= 2;
    };
    set(
        shared,
        ScheduledMeasure::new(measure, started_at, duration, beats_per_measure, 0, 0),
    );
}

/// オートランダムのテスト用に「いま鳴っているグリッド」の周回数と token だけを差し込む。
#[cfg(any(test, feature = "test-support"))]
pub fn set_grid_for_test(shared: &SharedPlaybackPosition, cycle: u64, grid_token: u64) {
    set(
        shared,
        ScheduledMeasure::new(
            0,
            Instant::now(),
            Duration::from_secs(3_600),
            4,
            cycle,
            grid_token,
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
        let position = ScheduledMeasure::new(2, start, Duration::from_secs(2), 4, 0, 0);

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
        let position = ScheduledMeasure::new(0, start, Duration::from_secs(2), 3, 0, 0);

        assert_eq!(
            position.current_beat(start - Duration::from_millis(1)),
            None
        );
        assert_eq!(position.current_beat(start + Duration::from_secs(2)), None);
    }
}
