use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use rodio::Source as _;
use rubberband_ffi::StretchProfile;

pub const TARGET_BPM: f64 = 120.0;
pub const MIN_TIME_RATIO: f64 = 0.8;
pub const MAX_TIME_RATIO: f64 = 1.25;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TargetBpm {
    pub bpm: f64,
    pub has_common_range: bool,
}

pub fn select_target_bpm(source_bpms: impl IntoIterator<Item = f64>) -> TargetBpm {
    let mut minimum = 0.0_f64;
    let mut maximum = f64::INFINITY;
    let mut has_source = false;
    for source_bpm in source_bpms {
        if !source_bpm.is_finite() || source_bpm <= 0.0 {
            return TargetBpm {
                bpm: TARGET_BPM,
                has_common_range: false,
            };
        }
        has_source = true;
        minimum = minimum.max(source_bpm / MAX_TIME_RATIO);
        maximum = maximum.min(source_bpm / MIN_TIME_RATIO);
    }
    if !has_source {
        return TargetBpm {
            bpm: TARGET_BPM,
            has_common_range: true,
        };
    }
    if minimum > maximum {
        return TargetBpm {
            bpm: TARGET_BPM,
            has_common_range: false,
        };
    }
    TargetBpm {
        bpm: TARGET_BPM.clamp(minimum, maximum),
        has_common_range: true,
    }
}

pub fn format_bpm(bpm: f64) -> String {
    let rounded = bpm.round();
    if (bpm - rounded).abs() < 0.000_000_1 {
        format!("{rounded:.0}")
    } else {
        let formatted = format!("{bpm:.2}");
        formatted
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

#[derive(Clone, Debug)]
pub struct PreparedAudio {
    samples: Arc<[f32]>,
    channels: u16,
    sample_rate: u32,
    info: PreparedAudioInfo,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PreparedAudioInfo {
    pub input_frames: usize,
    pub rubberband_output_frames: usize,
    pub output_frames: usize,
    pub channels: u16,
    pub sample_rate: u32,
    pub time_ratio: f64,
    pub profile: StretchProfile,
}

impl PreparedAudio {
    pub fn source(&self) -> PreparedAudioSource {
        PreparedAudioSource {
            audio: self.clone(),
            position: 0,
        }
    }

    pub fn info(&self) -> PreparedAudioInfo {
        self.info
    }

    #[cfg(test)]
    fn frames(&self) -> usize {
        self.samples.len() / usize::from(self.channels)
    }
}

pub struct PreparedAudioSource {
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

pub fn profile_for_category(category: Option<&str>) -> StretchProfile {
    if category.is_some_and(|category| category.trim().eq_ignore_ascii_case("drum")) {
        StretchProfile::Drum
    } else {
        StretchProfile::General
    }
}

pub fn time_ratio(source_bpm: f64, target_bpm: f64) -> Result<f64> {
    if !source_bpm.is_finite() || source_bpm <= 0.0 {
        anyhow::bail!("解析BPMが不正です: {source_bpm}");
    }
    if !target_bpm.is_finite() || target_bpm <= 0.0 {
        anyhow::bail!("target BPMが不正です: {target_bpm}");
    }
    let ratio = source_bpm / target_bpm;
    if !(MIN_TIME_RATIO..=MAX_TIME_RATIO).contains(&ratio) {
        anyhow::bail!(
            "BPM{source_bpm:.2}はBPM{}伸縮範囲外です（time ratio {ratio:.3}, allowed {MIN_TIME_RATIO}..={MAX_TIME_RATIO}）",
            format_bpm(target_bpm)
        );
    }
    Ok(ratio)
}

pub fn exceeds_time_ratio_limits(source_bpm: f64, target_bpm: f64) -> bool {
    if !source_bpm.is_finite() || source_bpm <= 0.0 || !target_bpm.is_finite() || target_bpm <= 0.0
    {
        return false;
    }
    !(MIN_TIME_RATIO..=MAX_TIME_RATIO).contains(&(source_bpm / target_bpm))
}

pub fn prepare_path<F>(
    path: &Path,
    source_bpm: Option<f64>,
    target_bpm: f64,
    category: Option<&str>,
    cancelled: F,
) -> Result<PreparedAudio>
where
    F: Fn() -> bool,
{
    let ratio = source_bpm
        .map(|source_bpm| time_ratio(source_bpm, target_bpm))
        .transpose()?
        .unwrap_or(1.0);
    if cancelled() {
        anyhow::bail!("音声準備をキャンセルしました");
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
    let profile = profile_for_category(category);
    let mut samples = if (ratio - 1.0).abs() < f64::EPSILON {
        input
    } else {
        rubberband_ffi::stretch_interleaved_with_cancel(
            &input,
            sample_rate,
            channels,
            ratio,
            profile,
            cancelled,
        )
        .with_context(|| {
            format!(
                "BPM{}へ伸縮できません: {}",
                format_bpm(target_bpm),
                path.display()
            )
        })?
    };
    let rubberband_output_frames = samples.len() / usize::from(channels);
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
        info: PreparedAudioInfo {
            input_frames,
            rubberband_output_frames,
            output_frames: expected_frames,
            channels,
            sample_rate,
            time_ratio: ratio,
            profile,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn target_bpm_is_closest_value_to_120_in_the_common_range() {
        assert_eq!(select_target_bpm([]).bpm, 120.0);
        assert_eq!(select_target_bpm([96.0, 150.0]).bpm, 120.0);
        assert_eq!(select_target_bpm([95.0]).bpm, 118.75);
        assert_eq!(select_target_bpm([160.0]).bpm, 128.0);
        assert_eq!(select_target_bpm([95.0, 100.0]).bpm, 118.75);
    }

    #[test]
    fn target_bpm_falls_back_to_120_without_a_common_range() {
        let target = select_target_bpm([60.0, 200.0]);
        assert_eq!(target.bpm, 120.0);
        assert!(!target.has_common_range);

        let invalid = select_target_bpm([f64::NAN]);
        assert_eq!(invalid.bpm, 120.0);
        assert!(!invalid.has_common_range);
    }

    #[test]
    fn time_ratio_limit_check_only_reports_finite_positive_outliers() {
        assert!(!exceeds_time_ratio_limits(96.0, 120.0));
        assert!(!exceeds_time_ratio_limits(150.0, 120.0));
        assert!(exceeds_time_ratio_limits(95.0, 120.0));
        assert!(exceeds_time_ratio_limits(151.0, 120.0));
        assert!(!exceeds_time_ratio_limits(f64::NAN, 120.0));
        assert!(!exceeds_time_ratio_limits(120.0, 0.0));
    }

    #[test]
    fn bpm_ratio_accepts_target_dependent_range() {
        assert_eq!(time_ratio(96.0, 120.0).unwrap(), 0.8);
        assert_eq!(time_ratio(99.0, 120.0).unwrap(), 0.825);
        assert_eq!(time_ratio(120.0, 120.0).unwrap(), 1.0);
        assert_eq!(time_ratio(150.0, 120.0).unwrap(), 1.25);
        assert_eq!(time_ratio(95.0, 118.75).unwrap(), 0.8);
        assert!(time_ratio(95.9, 120.0).is_err());
        assert!(time_ratio(150.1, 120.0).is_err());
        assert!(time_ratio(f64::NAN, 120.0).is_err());
        assert!(time_ratio(120.0, f64::NAN).is_err());
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

        let audio = prepare_path(&path, Some(99.0), 120.0, Some("drum"), || false).unwrap();
        assert_eq!(audio.channels, 2);
        assert_eq!(audio.sample_rate, 48_000);
        assert_eq!(audio.frames(), 3_960);
        assert_eq!(audio.info().input_frames, 4_800);
        assert!(audio.info().rubberband_output_frames > 0);
        assert_eq!(audio.info().output_frames, 3_960);
        assert_eq!(audio.info().channels, 2);
        assert_eq!(audio.info().sample_rate, 48_000);
        assert_eq!(audio.info().profile, StretchProfile::Drum);
        assert!((audio.info().time_ratio - 0.825).abs() < f64::EPSILON);
        assert!(audio.samples.iter().all(|sample| sample.is_finite()));

        let bypassed = prepare_path(&path, Some(120.0), 120.0, None, || false).unwrap();
        assert_eq!(bypassed.info().input_frames, 4_800);
        assert_eq!(bypassed.info().rubberband_output_frames, 4_800);
        assert_eq!(bypassed.info().output_frames, 4_800);
        assert_eq!(bypassed.info().time_ratio, 1.0);

        let one_shot = prepare_path(&path, None, 37.0, None, || false).unwrap();
        assert_eq!(one_shot.info().input_frames, 4_800);
        assert_eq!(one_shot.info().rubberband_output_frames, 4_800);
        assert_eq!(one_shot.info().output_frames, 4_800);
        assert_eq!(one_shot.info().time_ratio, 1.0);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn cancellation_is_checked_before_opening_the_wav() {
        let error =
            prepare_path(Path::new("missing.wav"), Some(120.0), 120.0, None, || true).unwrap_err();
        assert!(error.to_string().contains("キャンセル"));
    }
}
