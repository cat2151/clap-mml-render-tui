use crate::tui::loop_browser::playback::diagnostics::{StretchStatus, StretchStatusView};

pub(super) fn stretch_label(
    source_bpm: Option<f64>,
    target_bpm: f64,
    span_measures: usize,
    meter_numerator: u16,
    meter_denominator: u16,
    stretch_status: Option<&StretchStatusView>,
    exceeds_limits: bool,
) -> String {
    if source_bpm.is_none() {
        return match stretch_status.map(|view| &view.status) {
            None => "one-shot / ?".to_string(),
            Some(StretchStatus::Pending) => "one-shot / pending".to_string(),
            Some(StretchStatus::Ready { info, .. }) => {
                let seconds = crate::tui::loop_browser::playback::diagnostics::duration_seconds(
                    info.output_frames,
                    info.sample_rate,
                );
                format!("one-shot / bypass {seconds:.2}s")
            }
            Some(StretchStatus::Error(_)) => "one-shot / ERROR".to_string(),
        };
    }
    let source_bpm = source_bpm.unwrap();
    let source = crate::loop_time_stretch::format_bpm(source_bpm);
    let target = crate::loop_time_stretch::format_bpm(target_bpm);
    let Some(view) = stretch_status else {
        return if exceeds_limits {
            format!("NG r{:.2} {source}→{target}", source_bpm / target_bpm)
        } else {
            format!("? {source}→{target}")
        };
    };
    let source_bpm = view.source_bpm.unwrap_or(source_bpm);
    let source = crate::loop_time_stretch::format_bpm(source_bpm);
    let target = crate::loop_time_stretch::format_bpm(view.target_bpm);
    match view.status {
        StretchStatus::Pending => format!("… {source}→{target}"),
        StretchStatus::Ready { info, .. } => {
            let prepared_seconds =
                crate::tui::loop_browser::playback::diagnostics::duration_seconds(
                    info.output_frames,
                    info.sample_rate,
                );
            let scheduled_seconds = 60.0 / target_bpm * f64::from(meter_numerator) * 4.0
                / f64::from(meter_denominator)
                * span_measures as f64;
            let seconds = prepared_seconds.min(scheduled_seconds);
            if (info.time_ratio - 1.0).abs() < f64::EPSILON {
                format!("= {target} {seconds:.2}s")
            } else {
                format!("✓ {source}→{target} {seconds:.2}s")
            }
        }
        StretchStatus::Error(_) => {
            let ratio = source_bpm / view.target_bpm;
            if !(crate::loop_time_stretch::MIN_TIME_RATIO
                ..=crate::loop_time_stretch::MAX_TIME_RATIO)
                .contains(&ratio)
            {
                format!("NG r{ratio:.2} {source}→{target}")
            } else {
                format!("NG ERROR {source}→{target}")
            }
        }
    }
}
