//! Compact audio descriptors used by the loop browser waveform pane.

use serde::{Deserialize, Serialize};

mod analysis;

pub use analysis::analyze_file_with_progress;

pub const WAVEFORM_BINS_PER_MEASURE: usize = 32;
pub const SILENCE_DB_TENTHS: i16 = -960;
const FLUX_QUANTIZATION: f32 = 10_000.0;
const MOTION_QUANTIZATION: f32 = 1_000.0;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub struct LoopWaveform {
    /// RMS in tenths of a dBFS, clamped to -96..+24 dBFS.
    pub rms_db_tenths: Vec<i16>,
    /// L2 distance between adjacent normalized 48-band spectra, times 10,000.
    pub spectral_flux: Vec<u16>,
    /// Standard deviation of log2 spectral centroid, in thousandths of an octave.
    pub centroid_motion_millioctaves: u16,
}

impl LoopWaveform {
    pub fn silent(measures: usize) -> Self {
        let bins = measures.saturating_mul(WAVEFORM_BINS_PER_MEASURE);
        Self {
            rms_db_tenths: vec![SILENCE_DB_TENTHS; bins],
            spectral_flux: vec![0; bins],
            centroid_motion_millioctaves: 0,
        }
    }

    pub fn len(&self) -> usize {
        self.rms_db_tenths.len()
    }

    pub fn is_empty(&self) -> bool {
        self.rms_db_tenths.is_empty()
    }

    pub fn centroid_motion_octaves(&self) -> f32 {
        f32::from(self.centroid_motion_millioctaves) / MOTION_QUANTIZATION
    }
}

pub fn quantize_db(amplitude: f32) -> i16 {
    if amplitude <= 0.0 {
        return SILENCE_DB_TENTHS;
    }
    (20.0 * amplitude.log10() * 10.0)
        .round()
        .clamp(f32::from(SILENCE_DB_TENTHS), 240.0) as i16
}

pub fn quantize_flux(flux: f32) -> u16 {
    (flux * FLUX_QUANTIZATION)
        .round()
        .clamp(0.0, f32::from(u16::MAX)) as u16
}

pub fn quantize_motion(motion_octaves: f32) -> u16 {
    (motion_octaves * MOTION_QUANTIZATION)
        .round()
        .clamp(0.0, f32::from(u16::MAX)) as u16
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct WaveformDisplayScale {
    rms_low: i16,
    rms_high: i16,
    flux_low: u16,
    flux_high: u16,
}

impl Default for WaveformDisplayScale {
    fn default() -> Self {
        Self {
            rms_low: SILENCE_DB_TENTHS,
            rms_high: 0,
            flux_low: 0,
            flux_high: 1,
        }
    }
}

impl WaveformDisplayScale {
    pub fn from_waveforms(waveforms: &[LoopWaveform]) -> Self {
        let mut rms = Vec::new();
        let mut flux = Vec::new();
        for waveform in waveforms {
            for (&rms_value, &flux_value) in
                waveform.rms_db_tenths.iter().zip(&waveform.spectral_flux)
            {
                if rms_value > SILENCE_DB_TENTHS {
                    rms.push(rms_value);
                    flux.push(flux_value);
                }
            }
        }
        Self {
            rms_low: percentile(&mut rms, 10).unwrap_or(SILENCE_DB_TENTHS),
            rms_high: percentile(&mut rms, 90).unwrap_or(0),
            flux_low: percentile(&mut flux, 10).unwrap_or(0),
            flux_high: percentile(&mut flux, 90).unwrap_or(1),
        }
    }

    pub fn rms_rank(self, value: i16) -> f32 {
        normalized_rank(value, self.rms_low, self.rms_high)
    }

    pub fn flux_rank(self, value: u16) -> f32 {
        normalized_rank(value, self.flux_low, self.flux_high)
    }
}

fn percentile<T: Copy + Ord>(values: &mut [T], percent: usize) -> Option<T> {
    if values.is_empty() {
        return None;
    }
    values.sort_unstable();
    let index = (values.len() - 1).saturating_mul(percent) / 100;
    values.get(index).copied()
}

fn normalized_rank<T>(value: T, low: T, high: T) -> f32
where
    T: Copy + PartialOrd + Into<f64>,
{
    if high <= low {
        return 0.5;
    }
    (((value.into() - low.into()) / (high.into() - low.into())) as f32).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn display_scale_uses_tenth_and_ninetieth_percentiles() {
        let waveform = LoopWaveform {
            rms_db_tenths: (-10..=0).map(|value| value * 10).collect(),
            spectral_flux: (0..=10).map(|value| value * 100).collect(),
            centroid_motion_millioctaves: 0,
        };
        let scale = WaveformDisplayScale::from_waveforms(&[waveform]);

        assert_eq!(scale.rms_rank(-90), 0.0);
        assert_eq!(scale.rms_rank(-10), 1.0);
        assert_eq!(scale.flux_rank(100), 0.0);
        assert_eq!(scale.flux_rank(900), 1.0);
    }

    #[test]
    fn equal_descriptor_values_use_middle_rank() {
        let waveform = LoopWaveform {
            rms_db_tenths: vec![-120; 4],
            spectral_flux: vec![50; 4],
            centroid_motion_millioctaves: 0,
        };
        let scale = WaveformDisplayScale::from_waveforms(&[waveform]);
        assert_eq!(scale.rms_rank(-120), 0.5);
        assert_eq!(scale.flux_rank(50), 0.5);
    }
}
