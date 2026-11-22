//! CC1 pitch assessments keyed by the catalog's PatchRef.

use std::{collections::HashSet, fs, io::ErrorKind, path::Path};

use anyhow::{ensure, Context, Result};
use cmrt_core::PatchRef;
use serde::{Deserialize, Serialize};

pub const FILE_NAME: &str = "cc1.json";
pub const FORMAT_VERSION: u32 = 2;

#[derive(Debug, Serialize, Deserialize)]
pub struct Cache {
    pub format_version: u32,
    pub records: Vec<Record>,
}

impl Cache {
    pub fn load_next_to(catalog: &Path) -> Result<Option<Self>> {
        let path = catalog.with_file_name(FILE_NAME);
        let bytes = match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error).with_context(|| format!("{}", path.display())),
        };
        let cache: Self =
            serde_json::from_slice(&bytes).with_context(|| format!("{}", path.display()))?;
        ensure!(
            cache.format_version == FORMAT_VERSION,
            "{}: unsupported format_version {}",
            path.display(),
            cache.format_version
        );
        let mut keys = HashSet::with_capacity(cache.records.len());
        for record in &cache.records {
            ensure!(
                keys.insert((&record.patch.plugin, &record.patch.display)),
                "{}: duplicate CC1 PatchRef {:?}",
                path.display(),
                record.patch
            );
            ensure!(
                record.pitch_change_likely == record.observation.pitch_change_likely(),
                "{}: inconsistent CC1 assessment {:?}",
                path.display(),
                record.patch
            );
        }
        Ok(Some(cache))
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Record {
    pub patch: PatchRef,
    pub pitch_change_likely: bool,
    pub observation: Observation,
}

impl Record {
    pub fn new(patch: PatchRef, observation: Observation) -> Self {
        Self {
            patch,
            pitch_change_likely: observation.pitch_change_likely(),
            observation,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "method", rename_all = "snake_case")]
pub enum Observation {
    Routing {
        plugin_version: String,
        classification: RoutingClassification,
        evidence: String,
    },
    Audio {
        plugin_version: String,
        routing_unresolved: String,
        conditions: Conditions,
        classification: AudioClassification,
        frame_samples: usize,
        trials: [Option<PitchMetrics>; 2],
    },
    MeasurementFailed {
        routing_unresolved: String,
        plugin_version: Option<String>,
        error: String,
    },
}

impl Observation {
    fn pitch_change_likely(&self) -> bool {
        match self {
            Self::Routing { classification, .. } => matches!(
                classification,
                RoutingClassification::Vibrato | RoutingClassification::FixedPitchChange
            ),
            Self::Audio { classification, .. } => matches!(
                classification,
                AudioClassification::VibratoIncreaseCandidate
                    | AudioClassification::VibratoDecreaseCandidate
                    | AudioClassification::FixedPitchChangeCandidate
            ),
            Self::MeasurementFailed { .. } => false,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoutingClassification {
    Vibrato,
    FixedPitchChange,
    NonPitchRouting,
    Unconnected,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AudioClassification {
    VibratoIncreaseCandidate,
    VibratoDecreaseCandidate,
    FixedPitchChangeCandidate,
    NoClearPitchChange,
    PitchUncertain,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Conditions {
    pub sample_rate_hz: usize,
    pub duration_seconds: usize,
    pub block_samples: usize,
    pub preroll_blocks: usize,
    pub note_number: u8,
    pub velocity: u8,
    pub cc1_values: [u8; 2],
    pub fresh_instance_per_cc: bool,
    pub analysis: String,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct PitchMetrics {
    pub median_pitch_cents: f64,
    pub raw_pitch_span_cents: f64,
    pub pitch_span_cents: f64,
    pub sine_span_cents: f64,
    pub rate_hz: f64,
    pub sine_r2: f64,
}

#[cfg(test)]
mod tests;
