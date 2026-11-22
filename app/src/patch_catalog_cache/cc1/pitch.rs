//! Port of the exploratory pitch comparison used for the fresh-instance baseline.

use cmrt_tui_core::patch_catalog_cache::cc1::{AudioClassification, PitchMetrics};
use rustfft::{num_complex::Complex, FftPlanner};

use super::audio::SAMPLE_RATE;

const HOP: usize = 480;
const LONG_FRAME: usize = 4096;
const SHORT_FRAME: usize = 1024;
const MIN_RMS: f64 = 0.00001;

pub(super) fn compare(
    audio: [&[f32]; 2],
) -> (AudioClassification, usize, [Option<PitchMetrics>; 2]) {
    let mut fft = FftPlanner::new();
    let mono = audio.map(louder_channel);
    let mut frame = LONG_FRAME;
    let mut trials = track_pair(&mono, frame, &mut fft);
    if trials.iter().any(Option::is_none) {
        let short = track_pair(&mono, SHORT_FRAME, &mut fft);
        if short.iter().all(Option::is_some) {
            frame = SHORT_FRAME;
            trials = short;
        }
    }
    (classify(trials), frame, trials)
}

fn track_pair(
    mono: &[Vec<f64>; 2],
    frame: usize,
    planner: &mut FftPlanner<f64>,
) -> [Option<PitchMetrics>; 2] {
    let low = track(&mono[0], frame, planner);
    let high = if mono[0] == mono[1] {
        low
    } else {
        track(&mono[1], frame, planner)
    };
    [low, high]
}

fn classify(trials: [Option<PitchMetrics>; 2]) -> AudioClassification {
    use AudioClassification::*;
    let [Some(low), Some(high)] = trials else {
        return PitchUncertain;
    };
    let delta = high.pitch_span_cents - low.pitch_span_cents;
    let periodic = |trial: PitchMetrics| trial.sine_span_cents >= 20.0 && trial.sine_r2 >= 0.5;
    if delta >= 15.0 && periodic(high) {
        VibratoIncreaseCandidate
    } else if delta <= -15.0 && periodic(low) {
        VibratoDecreaseCandidate
    } else {
        let offset = (low.median_pitch_cents - high.median_pitch_cents).abs();
        let max_span = low.raw_pitch_span_cents.max(high.raw_pitch_span_cents);
        if offset < 25.0 || offset < 2.0 * max_span {
            return NoClearPitchChange;
        }
        // A pure change in relative harmonic level can make the tracker lock
        // onto a different integer multiple of the same fundamental.
        let ratio = (offset / 1200.0).exp2().round();
        if ratio >= 2.0 && (offset - 1200.0 * ratio.log2()).abs() <= 20.0 {
            return PitchUncertain;
        }
        FixedPitchChangeCandidate
    }
}

fn louder_channel(audio: &[f32]) -> Vec<f64> {
    let (stereo, _) = audio.as_chunks::<2>();
    let mut power = [0.0; 2];
    for pair in stereo {
        for channel in 0..2 {
            power[channel] += f64::from(pair[channel]).powi(2);
        }
    }
    let channel = usize::from(power[1] > power[0]);
    stereo.iter().map(|pair| f64::from(pair[channel])).collect()
}

fn track(signal: &[f64], frame: usize, planner: &mut FftPlanner<f64>) -> Option<PitchMetrics> {
    if signal.iter().all(|sample| sample.abs() < MIN_RMS) {
        return None;
    }
    let fft_len = 2 * frame;
    let forward = planner.plan_fft_forward(fft_len);
    let inverse = planner.plan_fft_inverse(fft_len);
    let mut scratch = vec![
        Complex::default();
        forward
            .get_inplace_scratch_len()
            .max(inverse.get_inplace_scratch_len())
    ];
    let mut spectrum = vec![Complex::default(); fft_len];
    let mut energy = vec![0.0; frame + 1];
    let mut nsdf = vec![0.0; 1201.min(frame / 2) - 16];
    let mut frames = Vec::new();
    let mut max_rms: f64 = 0.0;
    for (index, window) in signal.windows(frame).step_by(HOP).enumerate() {
        let rms = (dot(window, window) / frame as f64).sqrt();
        max_rms = max_rms.max(rms);
        let time = (index * HOP + frame / 2) as f64 / SAMPLE_RATE as f64;
        if !(0.15..=1.95).contains(&time) {
            continue;
        }
        let mean = window.iter().sum::<f64>() / frame as f64;
        spectrum.fill(Complex::default());
        for (i, &sample) in window.iter().enumerate() {
            let centered = sample - mean;
            spectrum[i].re = centered;
            energy[i + 1] = energy[i] + centered * centered;
        }
        forward.process_with_scratch(&mut spectrum, &mut scratch);
        for value in &mut spectrum {
            *value = Complex::new(value.norm_sqr(), 0.0);
        }
        inverse.process_with_scratch(&mut spectrum, &mut scratch);
        for (i, value) in nsdf.iter_mut().enumerate() {
            let lag = i + 16;
            let pair_energy = energy[frame - lag] + energy[frame] - energy[lag];
            *value = 2.0 * spectrum[lag].re / fft_len as f64 / pair_energy.max(1e-20);
        }
        frames.push((time, rms, pitch_cents(&nsdf)));
    }
    let selected = frames.len();
    let (time, cents): (Vec<_>, Vec<_>) = frames
        .into_iter()
        .filter(|(_, rms, _)| *rms >= MIN_RMS.max(max_rms * 0.03))
        .filter_map(|(time, _, pitch)| Some((time, pitch?)))
        .unzip();
    if cents.len() < 30 || (cents.len() as f64) < 0.7 * selected as f64 {
        return None;
    }
    let jumps = cents
        .windows(2)
        .filter(|pair| (pair[1] - pair[0]).abs() > 150.0)
        .count();
    if jumps as f64 / (cents.len() - 1) as f64 > 0.05 {
        return None;
    }
    Some(periodicity(&time, cents))
}

fn pitch_cents(nsdf: &[f64]) -> Option<f64> {
    let threshold = 0.85_f64.max(0.9 * nsdf.iter().copied().fold(f64::NEG_INFINITY, f64::max));
    let (index, peak) = nsdf
        .windows(3)
        .enumerate()
        .find(|(_, p)| p[1] > p[0] && p[1] >= p[2] && p[1] >= threshold)?;
    let curvature = peak[0] - 2.0 * peak[1] + peak[2];
    let offset = if curvature.abs() > 1e-20 {
        (0.5 * (peak[0] - peak[2]) / curvature).clamp(-0.5, 0.5)
    } else {
        0.0
    };
    let lag = (index + 17) as f64 + offset;
    Some(1200.0 * (SAMPLE_RATE as f64 / lag).log2())
}

fn periodicity(time: &[f64], mut cents: Vec<f64>) -> PitchMetrics {
    let mut raw = cents.clone();
    raw.sort_by(f64::total_cmp);
    let median = percentile(&raw, 0.5);
    let raw_span = percentile(&raw, 0.95) - percentile(&raw, 0.05);
    let mean_time = time.iter().sum::<f64>() / time.len() as f64;
    let centered_time: Vec<_> = time.iter().map(|t| t - mean_time).collect();
    let time_energy = dot(&centered_time, &centered_time);
    remove_trend(&mut cents, &centered_time, time_energy);
    let variance = dot(&cents, &cents).max(1e-20);
    let mut best = (0.0_f64, 2.0, 0.0);
    let mut sine = vec![0.0; time.len()];
    let mut cosine = vec![0.0; time.len()];
    for step in 0..=200 {
        let hz = 2.0 + f64::from(step) * 0.05;
        for (i, &t) in time.iter().enumerate() {
            (sine[i], cosine[i]) = (std::f64::consts::TAU * hz * t).sin_cos();
        }
        // Project both bases off the trend, then solve the two-coefficient least-squares fit.
        remove_trend(&mut sine, &centered_time, time_energy);
        remove_trend(&mut cosine, &centered_time, time_energy);
        let ss = dot(&sine, &sine);
        let cc = dot(&cosine, &cosine);
        let sc = dot(&sine, &cosine);
        let sy = dot(&sine, &cents);
        let cy = dot(&cosine, &cents);
        let determinant = ss * cc - sc * sc;
        let a = (cc * sy - sc * cy) / determinant;
        let b = (ss * cy - sc * sy) / determinant;
        let r2 = ((a * sy + b * cy) / variance).clamp(0.0, 1.0);
        if r2 > best.0 {
            best = (r2, hz, 2.0 * a.hypot(b));
        }
    }
    cents.sort_by(f64::total_cmp);
    PitchMetrics {
        median_pitch_cents: round(median, 100.0),
        raw_pitch_span_cents: round(raw_span, 100.0),
        pitch_span_cents: round(percentile(&cents, 0.95) - percentile(&cents, 0.05), 100.0),
        sine_span_cents: round(best.2, 100.0),
        rate_hz: round(best.1, 100.0),
        sine_r2: round(best.0, 1000.0),
    }
}

fn remove_trend(values: &mut [f64], time: &[f64], time_energy: f64) {
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let slope = dot(values, time) / time_energy;
    for (value, t) in values.iter_mut().zip(time) {
        *value -= mean + slope * t;
    }
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn percentile(sorted: &[f64], fraction: f64) -> f64 {
    let index = (sorted.len() - 1) as f64 * fraction;
    let lower = index.floor() as usize;
    sorted[lower] + index.fract() * (sorted[index.ceil() as usize] - sorted[lower])
}

fn round(value: f64, scale: f64) -> f64 {
    (value * scale).round() / scale
}

#[cfg(test)]
mod tests;
