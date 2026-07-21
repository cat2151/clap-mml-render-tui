use super::{quantize_db, quantize_flux, quantize_motion, LoopWaveform};
use anyhow::{Context, Result};
use rustfft::{num_complex::Complex32, FftPlanner};
use std::path::Path;

const SPECTRAL_BANDS: usize = 48;
const MIN_FREQUENCY_HZ: f32 = 20.0;

#[cfg(test)]
fn analyze_file(path: &Path, measures: usize) -> Result<LoopWaveform> {
    analyze_file_with_progress(path, measures, |_, _| {})
}

pub(crate) fn analyze_file_with_progress(
    path: &Path,
    measures: usize,
    mut progress: impl FnMut(usize, usize),
) -> Result<LoopWaveform> {
    let mut reader = hound::WavReader::open(path)
        .with_context(|| format!("波形用にWAVを開けません: {}", path.display()))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels);
    let samples = read_samples(&mut reader, spec)?;
    let frames = samples.len().checked_div(channels).unwrap_or(0);
    let bin_count = measures
        .checked_mul(super::WAVEFORM_BINS_PER_MEASURE)
        .ok_or_else(|| anyhow::anyhow!("波形の要素数が大きすぎます"))?;
    if channels == 0 || frames == 0 || bin_count == 0 {
        anyhow::bail!("波形を作成できないWAV形式です");
    }

    let mut planner = FftPlanner::<f32>::new();
    let mut rms_db_tenths = Vec::with_capacity(bin_count);
    let mut spectral_flux = Vec::with_capacity(bin_count);
    let mut centroids = Vec::with_capacity(bin_count);
    let mut previous_bands = None;
    for bin in 0..bin_count {
        let start = bin * frames / bin_count;
        let end = ((bin + 1) * frames / bin_count).max(start + 1).min(frames);
        let rms = rms(&samples, channels, start, end)?;
        let (centroid, bands) = spectrum(
            &samples,
            channels,
            start,
            end,
            spec.sample_rate,
            &mut planner,
        );
        let flux = previous_bands
            .as_ref()
            .map_or(0.0, |previous| spectral_flux_l2(previous, &bands));
        previous_bands = Some(bands);
        rms_db_tenths.push(quantize_db(rms));
        spectral_flux.push(quantize_flux(flux));
        if rms > 0.0 && centroid.is_finite() {
            centroids.push(centroid.max(MIN_FREQUENCY_HZ).log2());
        }
        progress(bin + 1, bin_count);
    }

    Ok(LoopWaveform {
        rms_db_tenths,
        spectral_flux,
        centroid_motion_millioctaves: quantize_motion(standard_deviation(&centroids)),
    })
}

fn read_samples(
    reader: &mut hound::WavReader<std::io::BufReader<std::fs::File>>,
    spec: hound::WavSpec,
) -> Result<Vec<f32>> {
    match spec.sample_format {
        hound::SampleFormat::Float => reader
            .samples::<f32>()
            .map(|sample| validate_sample(sample.context("WAV sampleを読めません")?))
            .collect(),
        hound::SampleFormat::Int => {
            if !(1..=32).contains(&spec.bits_per_sample) {
                anyhow::bail!("未対応の量子化bit数です: {}", spec.bits_per_sample);
            }
            let scale = ((1_i64 << (spec.bits_per_sample - 1)) - 1).max(1) as f32;
            reader
                .samples::<i32>()
                .map(|sample| {
                    validate_sample(sample.context("WAV sampleを読めません")? as f32 / scale)
                })
                .collect()
        }
    }
}

fn validate_sample(sample: f32) -> Result<f32> {
    if !sample.is_finite() {
        anyhow::bail!("WAV sampleに非有限値があります");
    }
    Ok(sample)
}

fn rms(samples: &[f32], channels: usize, start: usize, end: usize) -> Result<f32> {
    let range = start * channels..end * channels;
    let mut energy = 0.0_f64;
    let mut count = 0_usize;
    for &sample in samples.get(range).context("WAV frame範囲が不正です")? {
        energy += f64::from(sample) * f64::from(sample);
        count += 1;
    }
    Ok(if count == 0 {
        0.0
    } else {
        (energy / count as f64).sqrt() as f32
    })
}

fn spectrum(
    samples: &[f32],
    channels: usize,
    start: usize,
    end: usize,
    sample_rate: u32,
    planner: &mut FftPlanner<f32>,
) -> (f32, [f32; SPECTRAL_BANDS]) {
    let frame_count = end - start;
    let fft_len = frame_count.next_power_of_two().max(2);
    let fft = planner.plan_fft_forward(fft_len);
    let mut magnitudes = vec![0.0_f32; fft_len / 2 + 1];
    let mut buffer = vec![Complex32::new(0.0, 0.0); fft_len];
    for channel in 0..channels {
        buffer.fill(Complex32::new(0.0, 0.0));
        for frame in 0..frame_count {
            let window = if frame_count <= 1 {
                1.0
            } else {
                let phase = std::f32::consts::TAU * frame as f32 / (frame_count - 1) as f32;
                0.5 - 0.5 * phase.cos()
            };
            buffer[frame].re = samples[(start + frame) * channels + channel] * window;
        }
        fft.process(&mut buffer);
        for (magnitude, value) in magnitudes.iter_mut().zip(&buffer) {
            *magnitude += value.norm() / channels as f32;
        }
    }

    descriptors_from_magnitudes(&magnitudes, sample_rate, fft_len)
}

fn descriptors_from_magnitudes(
    magnitudes: &[f32],
    sample_rate: u32,
    fft_len: usize,
) -> (f32, [f32; SPECTRAL_BANDS]) {
    let nyquist = sample_rate as f32 / 2.0;
    let log_range = (nyquist / MIN_FREQUENCY_HZ).max(1.0).ln();
    let mut bands = [0.0_f32; SPECTRAL_BANDS];
    let mut magnitude_sum = 0.0_f32;
    let mut weighted_frequency = 0.0_f32;
    for (index, &magnitude) in magnitudes.iter().enumerate() {
        let frequency = index as f32 * sample_rate as f32 / fft_len as f32;
        if frequency < MIN_FREQUENCY_HZ || magnitude <= 0.0 {
            continue;
        }
        magnitude_sum += magnitude;
        weighted_frequency += frequency * magnitude;
        let band = if log_range == 0.0 {
            0
        } else {
            ((frequency / MIN_FREQUENCY_HZ).ln() / log_range * SPECTRAL_BANDS as f32).floor()
                as usize
        }
        .min(SPECTRAL_BANDS - 1);
        bands[band] += magnitude;
    }
    if magnitude_sum > 0.0 {
        for band in &mut bands {
            *band /= magnitude_sum;
        }
        (weighted_frequency / magnitude_sum, bands)
    } else {
        (MIN_FREQUENCY_HZ, bands)
    }
}

fn spectral_flux_l2(previous: &[f32; SPECTRAL_BANDS], current: &[f32; SPECTRAL_BANDS]) -> f32 {
    previous
        .iter()
        .zip(current)
        .map(|(left, right)| (right - left).powi(2))
        .sum::<f32>()
        .sqrt()
}

fn standard_deviation(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mean = values.iter().sum::<f32>() / values.len() as f32;
    (values
        .iter()
        .map(|value| (value - mean).powi(2))
        .sum::<f32>()
        / values.len() as f32)
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_wav(label: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!(
            "cmrt_waveform_{label}_{}_{}.wav",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn write_sine(path: &Path, amplitude: f32, frequencies: &[f32]) {
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 8_000,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };
        let mut writer = hound::WavWriter::create(path, spec).unwrap();
        for frame in 0..8_000 {
            let frequency = frequencies[frame * frequencies.len() / 8_000];
            let phase = std::f32::consts::TAU * frequency * frame as f32 / 8_000.0;
            writer.write_sample(amplitude * phase.sin()).unwrap();
        }
        writer.finalize().unwrap();
    }

    #[test]
    fn extracts_absolute_rms_and_spectral_motion() {
        let steady = temp_wav("steady");
        let moving = temp_wav("moving");
        write_sine(&steady, 0.5, &[220.0]);
        write_sine(&moving, 0.25, &[110.0, 1_760.0]);

        let steady_waveform = analyze_file(&steady, 1).unwrap();
        let moving_waveform = analyze_file(&moving, 1).unwrap();
        assert_eq!(
            steady_waveform.len(),
            crate::loop_waveform::WAVEFORM_BINS_PER_MEASURE
        );
        assert!(steady_waveform.rms_db_tenths[0] > moving_waveform.rms_db_tenths[0]);
        assert!(moving_waveform.centroid_motion_millioctaves > 500);
        assert!(steady_waveform.centroid_motion_millioctaves < 100);
        assert!(moving_waveform.spectral_flux.iter().any(|&flux| flux > 0));

        std::fs::remove_file(steady).unwrap();
        std::fs::remove_file(moving).unwrap();
    }

    #[test]
    fn silence_remains_at_floor() {
        let path = temp_wav("silence");
        write_sine(&path, 0.0, &[220.0]);
        let waveform = analyze_file(&path, 1).unwrap();
        assert!(waveform
            .rms_db_tenths
            .iter()
            .all(|&value| value == super::super::SILENCE_DB_TENTHS));
        assert!(waveform.spectral_flux.iter().all(|&value| value == 0));
        std::fs::remove_file(path).unwrap();
    }
}
