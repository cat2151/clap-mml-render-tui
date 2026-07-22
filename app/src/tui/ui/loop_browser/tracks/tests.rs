use super::*;
use crate::loop_browser::time_stretch::PreparedAudioInfo;
use crate::tui::loop_browser::playback::diagnostics::{StretchStatus, StretchStatusView};

#[test]
fn cropped_prev_marker_reports_its_scheduled_output_duration() {
    let view = StretchStatusView {
        source_bpm: Some(120.0),
        target_bpm: 120.0,
        status: StretchStatus::Ready {
            info: PreparedAudioInfo {
                input_frames: 176_400,
                rubberband_output_frames: 176_400,
                output_frames: 176_400,
                channels: 2,
                sample_rate: 44_100,
                time_ratio: 1.0,
                profile: rubberband_ffi::StretchProfile::General,
            },
            cache_hit: false,
        },
    };

    assert_eq!(
        stretch_label(Some(120.0), 120.0, 1, 4, 4, Some(&view), false),
        "= 120 2.00s"
    );
}

#[test]
fn one_shot_reports_bypass_without_a_fake_bpm() {
    let view = StretchStatusView {
        source_bpm: None,
        target_bpm: 128.0,
        status: StretchStatus::Ready {
            info: PreparedAudioInfo {
                input_frames: 24_000,
                rubberband_output_frames: 24_000,
                output_frames: 24_000,
                channels: 1,
                sample_rate: 48_000,
                time_ratio: 1.0,
                profile: rubberband_ffi::StretchProfile::General,
            },
            cache_hit: false,
        },
    };

    assert_eq!(
        stretch_label(None, 128.0, 1, 4, 4, Some(&view), false),
        "one-shot / bypass 0.50s"
    );
}
