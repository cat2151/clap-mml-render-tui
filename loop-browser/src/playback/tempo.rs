use std::time::Duration;

use crate::{LoopPlaybackClip, LoopPlaybackGrid};
use cmrt_loop_browser_domain::time_stretch::{select_target_bpm, TargetBpm};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MeasureTiming {
    pub duration: Duration,
    pub beats_per_measure: usize,
}

pub fn grid_target_bpm(grid: &LoopPlaybackGrid) -> TargetBpm {
    select_target_bpm(
        grid.iter()
            .flatten()
            .filter_map(Option::as_ref)
            .filter_map(LoopPlaybackClip::source_bpm),
    )
}

pub fn measure_duration(grid: &LoopPlaybackGrid, target_bpm: f64) -> Duration {
    measure_timing(grid, target_bpm).duration
}

pub fn measure_timing(grid: &LoopPlaybackGrid, target_bpm: f64) -> MeasureTiming {
    let Some(clip) = grid_tempo_clip(grid) else {
        let seconds = 60.0 / target_bpm * 4.0;
        return MeasureTiming {
            duration: Duration::from_secs_f64(seconds).max(Duration::from_millis(1)),
            beats_per_measure: 4,
        };
    };
    let seconds = 60.0 / target_bpm * f64::from(clip.meter_numerator) * 4.0
        / f64::from(clip.meter_denominator);
    let duration = if seconds.is_finite() && seconds > 0.0 {
        Duration::from_secs_f64(seconds).max(Duration::from_millis(1))
    } else {
        Duration::from_secs(2)
    };
    MeasureTiming {
        duration,
        beats_per_measure: usize::from(clip.meter_numerator).max(1),
    }
}

fn grid_tempo_clip(grid: &LoopPlaybackGrid) -> Option<&LoopPlaybackClip> {
    let measures = grid.iter().map(Vec::len).max().unwrap_or(0);
    for measure in 0..measures {
        for track in grid {
            if let Some(clip) = track
                .get(measure)
                .and_then(Option::as_ref)
                .filter(|clip| !clip.is_one_shot() && clip.bpm.is_some())
            {
                return Some(clip);
            }
        }
    }
    None
}
