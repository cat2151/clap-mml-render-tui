use std::time::Duration;

use crate::loop_time_stretch::{select_target_bpm, TargetBpm};
use crate::tui::loop_browser::{LoopPlaybackClip, LoopPlaybackGrid};

pub(super) fn grid_target_bpm(grid: &LoopPlaybackGrid) -> TargetBpm {
    select_target_bpm(
        grid.iter()
            .flatten()
            .filter_map(Option::as_ref)
            .map(|clip| clip.bpm),
    )
}

pub(super) fn measure_duration(grid: &LoopPlaybackGrid, target_bpm: f64) -> Duration {
    let Some(clip) = grid_tempo_clip(grid) else {
        return Duration::from_millis(1);
    };
    let seconds = 60.0 / target_bpm * f64::from(clip.meter_numerator) * 4.0
        / f64::from(clip.meter_denominator);
    if seconds.is_finite() && seconds > 0.0 {
        Duration::from_secs_f64(seconds).max(Duration::from_millis(1))
    } else {
        Duration::from_secs(2)
    }
}

fn grid_tempo_clip(grid: &LoopPlaybackGrid) -> Option<&LoopPlaybackClip> {
    let measures = grid.iter().map(Vec::len).max().unwrap_or(0);
    for measure in 0..measures {
        for track in grid {
            if let Some(clip) = track.get(measure).and_then(Option::as_ref) {
                return Some(clip);
            }
        }
    }
    None
}
