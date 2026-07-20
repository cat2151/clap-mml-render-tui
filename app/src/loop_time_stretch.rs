use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use rodio::Source as _;
use rubberband_ffi::StretchProfile;

pub(crate) const TARGET_BPM: f64 = 120.0;
pub(crate) const MIN_TIME_RATIO: f64 = 0.8;
pub(crate) const MAX_TIME_RATIO: f64 = 1.25;

#[derive(Clone, Debug)]
pub(crate) struct PreparedAudio {
    samples: Arc<[f32]>,
    channels: u16,
    sample_rate: u32,
}

impl PreparedAudio {
    pub(crate) fn source(&self) -> PreparedAudioSource {
        PreparedAudioSource {
            audio: self.clone(),
            position: 0,
        }
    }

    #[cfg(test)]
    fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels)
    }
}

pub(crate) struct PreparedAudioSource {
    audio: PreparedAudio,
    position: usize,
}

impl Iterator for PreparedAudioSource {
    type Item = f32;

    fn next(&mut self) -> Option<Self::Item> {
        let sample = self.audio.samples.get(self.position).copied()?;
        self.position += 1;
        Some(sample)
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let remaining = self.audio.samples.len().saturating_sub(self.position);
        (remaining, Some(remaining))
    }
}

impl ExactSizeIterator for PreparedAudioSource {}

impl rodio::Source for PreparedAudioSource {
    fn current_frame_len(&self) -> Option<usize> {
        Some(self.audio.samples.len().saturating_sub(self.position))
    }

    fn channels(&self) -> u16 {
        self.audio.channels
    }

    fn sample_rate(&self) -> u32 {
        self.audio.sample_rate
    }

    fn total_duration(&self) -> Option<Duration> {
        let frames = self.audio.samples.len() / usize::from(self.audio.channels);
        Some(Duration::from_secs_f64(
            frames as f64 / f64::from(self.audio.sample_rate),
        ))
    }
}

pub(crate) fn profile_for_category(category: Option<&str>) -> StretchProfile {
    if category.is_some_and(|category| category.trim().eq_ignore_ascii_case("drum")) {
        StretchProfile::Drum
    } else {
        StretchProfile::General
    }
}

pub(crate) fn time_ratio(source_bpm: f64) -> Result<f64> {
    if !source_bpm.is_finite() || source_bpm <= 0.0 {
        anyhow::bail!("解析BPMが不正です: {source_bpm}");
    }
    let ratio = source_bpm / TARGET_BPM;
    if !(MIN_TIME_RATIO..=MAX_TIME_RATIO).contains(&ratio) {
        anyhow::bail!(
            "BPM{source_bpm:.2}はBPM120伸縮範囲外です（time ratio {ratio:.3}, allowed {MIN_TIME_RATIO}..={MAX_TIME_RATIO}）"
        );
    }
    Ok(ratio)
}

pub(crate) fn prepare_path<F>(
    path: &Path,
    source_bpm: f64,
    category: Option<&str>,
    cancelled: F,
) -> Result<PreparedAudio>
where
    F: Fn() -> bool,
{
    let ratio = time_ratio(source_bpm)?;
    if cancelled() {
        anyhow::bail!("BPM120変換をキャンセルしました");
    }
    let file = File::open(path).with_context(|| format!("WAVを開けません: {}", path.display()))?;
    let source = rodio::Decoder::new(BufReader::new(file))
        .with_context(|| format!("WAVをdecodeできません: {}", path.display()))?;
    let channels = source.channels();
    let sample_rate = source.sample_rate();
    let input = source.convert_samples::<f32>().collect::<Vec<_>>();
    if input.is_empty() {
        anyhow::bail!("WAVにsampleがありません: {}", path.display());
    }
    let input_frames = input.len() / usize::from(channels);
    let mut samples = if (ratio - 1.0).abs() < f64::EPSILON {
        input
    } else {
        rubberband_ffi::stretch_interleaved_with_cancel(
            &input,
            sample_rate,
            channels,
            ratio,
            profile_for_category(category),
            cancelled,
        )
        .with_context(|| format!("BPM120へ伸縮できません: {}", path.display()))?
    };
    let expected_frames = (input_frames as f64 * ratio).round() as usize;
    let expected_samples = expected_frames
        .checked_mul(usize::from(channels))
        .ok_or_else(|| anyhow::anyhow!("伸縮後のsample数が大きすぎます"))?;
    samples.resize(expected_samples, 0.0);
    samples.truncate(expected_samples);
    Ok(PreparedAudio {
        samples: samples.into(),
        channels,
        sample_rate,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bpm_ratio_accepts_initial_ux_range() {
        assert_eq!(time_ratio(96.0).unwrap(), 0.8);
        assert_eq!(time_ratio(99.0).unwrap(), 0.825);
        assert_eq!(time_ratio(120.0).unwrap(), 1.0);
        assert_eq!(time_ratio(150.0).unwrap(), 1.25);
        assert!(time_ratio(95.9).is_err());
        assert!(time_ratio(150.1).is_err());
        assert!(time_ratio(f64::NAN).is_err());
    }

    #[test]
    fn drum_category_is_case_insensitive_and_trimmed() {
        assert_eq!(profile_for_category(Some("drum")), StretchProfile::Drum);
        assert_eq!(profile_for_category(Some(" Drum ")), StretchProfile::Drum);
        assert_eq!(profile_for_category(Some("bass")), StretchProfile::General);
        assert_eq!(profile_for_category(None), StretchProfile::General);
    }

    #[test]
    fn wav_is_decoded_and_stretched_from_99_to_120() {
        let path = std::env::temp_dir().join(format!(
            "cmrt-rubberband-{}-{}.wav",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        let spec = hound::WavSpec {
            channels: 2,
            sample_rate: 48_000,
            bits_per_sample: 16,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        for frame in 0..4_800 {
            let sample = (((frame as f32 / 48.0) * std::f32::consts::TAU).sin()
                * f32::from(i16::MAX)) as i16;
            writer.write_sample(sample).unwrap();
            writer.write_sample(-sample).unwrap();
        }
        writer.finalize().unwrap();

        let audio = prepare_path(&path, 99.0, Some("drum"), || false).unwrap();
        assert_eq!(audio.channels, 2);
        assert_eq!(audio.sample_rate, 48_000);
        assert_eq!(audio.frames(), 3_960);
        assert!(audio.samples.iter().all(|sample| sample.is_finite()));
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn cancellation_is_checked_before_opening_the_wav() {
        let error = prepare_path(Path::new("missing.wav"), 120.0, None, || true).unwrap_err();
        assert!(error.to_string().contains("キャンセル"));
    }
}
