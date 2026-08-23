//! Patchを2つのlive instanceへ読み込み、warmup後の所要時間を計測する。

use std::collections::BTreeMap;
use std::io::Write as _;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use cmrt_runtime::Config;
use cmrt_tui_core::patch_load::PatchLoadMeasurement;

pub(super) fn collect_patch_load_measurements(
    cfg: &Config,
    pairs: &[(String, String)],
) -> Result<BTreeMap<String, PatchLoadMeasurement>> {
    if pairs.is_empty() {
        return Ok(BTreeMap::new());
    }
    let supervisor =
        cmrt_realtime_play::RealtimePlayServerSupervisor::with_live_instance_count(cfg, 2);
    supervisor
        .start_owned_for_fast_midi()
        .context("patch load計測用realtime play serverを起動できません")?;
    let total = pairs.len();
    let progress_started = Instant::now();
    Ok(measure_patch_loads(
        pairs,
        |instance_id, patch| supervisor.prepare_live_patch(instance_id, Some(patch)),
        Instant::now,
        |index, patch| {
            print!("[{index}/{total}] {patch} ... ");
            let _ = std::io::stdout().flush();
        },
        |index, _patch, measurement| {
            let eta = estimate_eta(progress_started.elapsed(), index, total);
            println!(
                "first={} second={} ETA={}",
                measurement
                    .first_load_error
                    .as_deref()
                    .map_or("ok".to_string(), |error| format!("error: {error}")),
                match (
                    measurement.second_load_ms,
                    measurement.second_load_error.as_deref(),
                ) {
                    (Some(ms), _) => format!("{ms}ms"),
                    (None, Some(error)) => format!("error: {error}"),
                    (None, None) => "error: no measurement".to_string(),
                },
                format_eta(eta),
            );
        },
    ))
}

pub(super) fn measure_patch_loads<L, N, S, P>(
    pairs: &[(String, String)],
    mut load: L,
    mut now: N,
    mut report_started: S,
    mut report: P,
) -> BTreeMap<String, PatchLoadMeasurement>
where
    L: FnMut(u8, &str) -> Result<()>,
    N: FnMut() -> Instant,
    S: FnMut(usize, &str),
    P: FnMut(usize, &str, &PatchLoadMeasurement),
{
    let mut measurements = BTreeMap::new();
    for (offset, (display, _)) in pairs.iter().enumerate() {
        report_started(offset + 1, display);
        let first_load_error = load(0, display).err().map(|error| format!("{error:#}"));
        let second_started = now();
        let second_result = load(1, display);
        let second_elapsed = now().duration_since(second_started);
        let (second_load_ms, second_load_error) = match second_result {
            Ok(()) => (
                Some(u64::try_from(second_elapsed.as_millis()).unwrap_or(u64::MAX)),
                None,
            ),
            Err(error) => (None, Some(format!("{error:#}"))),
        };
        let measurement = PatchLoadMeasurement {
            second_load_ms,
            first_load_error,
            second_load_error,
        };
        report(offset + 1, display, &measurement);
        measurements.insert(display.clone(), measurement);
    }
    measurements
}

pub(super) fn estimate_eta(elapsed: Duration, completed: usize, total: usize) -> Duration {
    let remaining = total.saturating_sub(completed);
    if completed == 0 || remaining == 0 {
        return Duration::ZERO;
    }
    elapsed.mul_f64(remaining as f64 / completed as f64)
}

pub(super) fn format_eta(eta: Duration) -> String {
    let seconds = eta.as_secs();
    format!("{}分{:02}秒", seconds / 60, seconds % 60)
}
