use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use rubberband_ffi::{StretchProfile, GIT_REVISION};

use crate::{LoopGridChange, LoopPlaybackClip, LoopPlaybackGrid};
use cmrt_loop_browser_domain::time_stretch::{profile_for_category, PreparedAudioInfo, TargetBpm};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
struct StretchKey {
    path: PathBuf,
    source_bpm_bits: Option<u64>,
    target_bpm_bits: u64,
    profile: StretchProfile,
}

impl StretchKey {
    fn new(clip: &LoopPlaybackClip, target_bpm: f64) -> Self {
        Self {
            path: clip.path.clone(),
            source_bpm_bits: clip.source_bpm().map(f64::to_bits),
            target_bpm_bits: target_bpm.to_bits(),
            profile: profile_for_category(clip.category.as_deref()),
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum StretchStatus {
    Pending,
    Ready {
        info: PreparedAudioInfo,
        cache_hit: bool,
    },
    Error(String),
}

#[derive(Clone, Debug)]
pub struct StretchStatusView {
    pub source_bpm: Option<f64>,
    pub target_bpm: f64,
    pub status: StretchStatus,
}

#[derive(Clone, Debug)]
pub struct LoopStretchDiagnostics {
    generation: u64,
    target_bpm: TargetBpm,
    reason: LoopGridChange,
    entries: HashMap<StretchKey, StretchStatus>,
}

impl Default for LoopStretchDiagnostics {
    fn default() -> Self {
        Self {
            generation: 0,
            target_bpm: TargetBpm {
                bpm: cmrt_loop_browser_domain::time_stretch::TARGET_BPM,
                has_common_range: true,
            },
            reason: LoopGridChange::Initial,
            entries: HashMap::new(),
        }
    }
}

impl LoopStretchDiagnostics {
    pub fn begin(
        &mut self,
        generation: u64,
        reason: LoopGridChange,
        grid: &LoopPlaybackGrid,
        target_bpm: TargetBpm,
    ) {
        self.generation = generation;
        self.reason = reason;
        self.target_bpm = target_bpm;
        self.entries.clear();
        for clip in grid.iter().flatten().filter_map(Option::as_ref) {
            self.entries
                .entry(StretchKey::new(clip, target_bpm.bpm))
                .or_insert(StretchStatus::Pending);
        }
    }

    pub fn set_status(
        &mut self,
        generation: u64,
        clip: &LoopPlaybackClip,
        target_bpm: f64,
        status: StretchStatus,
    ) {
        if self.generation == generation {
            self.entries
                .insert(StretchKey::new(clip, target_bpm), status);
        }
    }

    pub fn status_for(
        &self,
        clip: &LoopPlaybackClip,
        target_bpm: f64,
    ) -> Option<StretchStatusView> {
        self.entries
            .get(&StretchKey::new(clip, target_bpm))
            .cloned()
            .map(|status| StretchStatusView {
                source_bpm: clip.source_bpm(),
                target_bpm,
                status,
            })
    }
}

pub type SharedLoopStretchDiagnostics = Arc<Mutex<LoopStretchDiagnostics>>;

pub fn new_shared() -> SharedLoopStretchDiagnostics {
    Arc::new(Mutex::new(LoopStretchDiagnostics::default()))
}

pub fn profile_label(profile: StretchProfile) -> &'static str {
    match profile {
        StretchProfile::Drum => "drum/R2",
        StretchProfile::General => "general/R3",
    }
}

pub fn duration_seconds(frames: usize, sample_rate: u32) -> f64 {
    frames as f64 / f64::from(sample_rate)
}

pub fn log_event(message: impl AsRef<str>) {
    #[cfg(not(test))]
    crate::log_line(&format!("loop-playback: {}", message.as_ref()));
    #[cfg(test)]
    let _ = message;
}

pub fn path_label(path: &Path) -> String {
    path.to_string_lossy().replace('"', "\\\"")
}

pub fn rubberband_revision() -> &'static str {
    GIT_REVISION
}

#[cfg(test)]
mod tests {
    use super::*;

    fn clip() -> LoopPlaybackClip {
        LoopPlaybackClip {
            path: PathBuf::from("kick.wav"),
            span_measures: 1,
            kind: cmrt_loop_domain::loop_wav_analysis::LoopWavKind::Loop,
            bpm: Some(100.0),
            category: Some("drum".to_string()),
            meter_numerator: 4,
            meter_denominator: 4,
        }
    }

    #[test]
    fn stale_generation_cannot_replace_current_status() {
        let clip = clip();
        let grid = vec![vec![Some(clip.clone())]];
        let target = TargetBpm {
            bpm: 120.0,
            has_common_range: true,
        };
        let mut diagnostics = LoopStretchDiagnostics::default();
        diagnostics.begin(2, LoopGridChange::Random, &grid, target);

        diagnostics.set_status(1, &clip, 120.0, StretchStatus::Error("stale".to_string()));

        assert!(matches!(
            diagnostics.status_for(&clip, 120.0).unwrap().status,
            StretchStatus::Pending
        ));
    }

    #[test]
    fn ready_status_preserves_waveform_and_cache_details() {
        let clip = clip();
        let grid = vec![vec![Some(clip.clone())]];
        let target = TargetBpm {
            bpm: 120.0,
            has_common_range: true,
        };
        let info = PreparedAudioInfo {
            input_frames: 48_000,
            rubberband_output_frames: 40_100,
            output_frames: 40_000,
            channels: 2,
            sample_rate: 48_000,
            time_ratio: 100.0 / 120.0,
            profile: StretchProfile::Drum,
        };
        let mut diagnostics = LoopStretchDiagnostics::default();
        diagnostics.begin(3, LoopGridChange::Pad('c'), &grid, target);
        diagnostics.set_status(
            3,
            &clip,
            120.0,
            StretchStatus::Ready {
                info,
                cache_hit: true,
            },
        );

        assert!(matches!(
            diagnostics.status_for(&clip, 120.0).unwrap().status,
            StretchStatus::Ready {
                info: actual,
                cache_hit: true,
            } if actual == info
        ));
    }
}
