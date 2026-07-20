//! RIFF/WAVE loop metadata analysis used by `scan-loops`.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::Path;

const RIFF_HEADER_SIZE: u64 = 12;
const CHUNK_HEADER_SIZE: u64 = 8;
const ACID_PAYLOAD_SIZE: u32 = 24;
const ACID_FLAG_ONE_SHOT: u32 = 1;

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum LoopAnalysisSource {
    Acid,
    DurationEstimate,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub(crate) struct LoopWavAnalysis {
    pub(crate) duration_seconds: f64,
    pub(crate) bpm: f64,
    pub(crate) beats: u32,
    pub(crate) meter_numerator: u16,
    pub(crate) meter_denominator: u16,
    pub(crate) measures: usize,
    pub(crate) source: LoopAnalysisSource,
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct TempoCandidate {
    bpm: f64,
    beats: u32,
    measures: usize,
}

#[derive(Clone, Copy, Debug)]
struct AcidMetadata {
    flags: u32,
    beats: u32,
    meter_denominator: u16,
    meter_numerator: u16,
    tempo: f32,
}

pub(crate) fn analyze_file(path: &Path) -> Result<LoopWavAnalysis> {
    let mut file =
        File::open(path).with_context(|| format!("WAVを開けません: {}", path.display()))?;
    analyze_reader(&mut file).with_context(|| format!("WAVを解析できません: {}", path.display()))
}

fn analyze_reader(reader: &mut (impl Read + Seek)) -> Result<LoopWavAnalysis> {
    let actual_len = reader.seek(SeekFrom::End(0))?;
    reader.seek(SeekFrom::Start(0))?;
    let mut header = [0_u8; RIFF_HEADER_SIZE as usize];
    reader
        .read_exact(&mut header)
        .context("RIFF headerが短すぎます")?;
    if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
        anyhow::bail!("RIFF/WAVE形式ではありません");
    }
    let riff_size = u32::from_le_bytes(header[4..8].try_into().unwrap()) as u64;
    let riff_end = riff_size
        .checked_add(8)
        .ok_or_else(|| anyhow::anyhow!("RIFF sizeが大きすぎます"))?;
    if riff_end > actual_len || riff_end < RIFF_HEADER_SIZE {
        anyhow::bail!("RIFF sizeがfile境界外です: declared={riff_end}, actual={actual_len}");
    }

    let mut byte_rate = None;
    let mut data_bytes = 0_u64;
    let mut acid = None;
    let mut offset = RIFF_HEADER_SIZE;
    while offset + CHUNK_HEADER_SIZE <= riff_end {
        reader.seek(SeekFrom::Start(offset))?;
        let mut chunk_header = [0_u8; CHUNK_HEADER_SIZE as usize];
        reader.read_exact(&mut chunk_header)?;
        let id = &chunk_header[0..4];
        let size = u32::from_le_bytes(chunk_header[4..8].try_into().unwrap());
        let payload = offset + CHUNK_HEADER_SIZE;
        let padded_size = u64::from(size) + u64::from(size % 2);
        let next = payload
            .checked_add(padded_size)
            .ok_or_else(|| anyhow::anyhow!("WAV chunk sizeが大きすぎます"))?;
        if next > riff_end {
            anyhow::bail!(
                "WAV chunkがRIFF境界外です: id={}, offset={offset}, size={size}",
                String::from_utf8_lossy(id)
            );
        }

        match id {
            b"fmt " if size >= 16 => {
                let mut fmt = [0_u8; 16];
                reader.read_exact(&mut fmt)?;
                let rate = u32::from_le_bytes(fmt[8..12].try_into().unwrap());
                if rate > 0 {
                    byte_rate = Some(rate);
                }
            }
            b"data" => data_bytes = data_bytes.saturating_add(u64::from(size)),
            b"acid" if size >= ACID_PAYLOAD_SIZE => {
                let mut bytes = [0_u8; ACID_PAYLOAD_SIZE as usize];
                reader.read_exact(&mut bytes)?;
                acid = Some(AcidMetadata {
                    flags: u32::from_le_bytes(bytes[0..4].try_into().unwrap()),
                    beats: u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
                    meter_denominator: u16::from_le_bytes(bytes[16..18].try_into().unwrap()),
                    meter_numerator: u16::from_le_bytes(bytes[18..20].try_into().unwrap()),
                    tempo: f32::from_le_bytes(bytes[20..24].try_into().unwrap()),
                });
            }
            _ => {}
        }
        offset = next;
    }
    if offset != riff_end {
        anyhow::bail!("RIFF末尾に不完全なchunk headerがあります");
    }

    let byte_rate = byte_rate.ok_or_else(|| anyhow::anyhow!("fmt chunkにbyte rateがありません"))?;
    if data_bytes == 0 {
        anyhow::bail!("data chunkが空です");
    }
    let duration_seconds = data_bytes as f64 / f64::from(byte_rate);
    if !duration_seconds.is_finite() || duration_seconds <= 0.0 {
        anyhow::bail!("WAVの再生時間が不正です");
    }

    if let Some(acid) = acid.filter(valid_acid) {
        let measures = (acid.beats as usize)
            .div_ceil(usize::from(acid.meter_numerator))
            .max(1);
        return Ok(LoopWavAnalysis {
            duration_seconds,
            bpm: f64::from(acid.tempo),
            beats: acid.beats,
            meter_numerator: acid.meter_numerator,
            meter_denominator: acid.meter_denominator,
            measures,
            source: LoopAnalysisSource::Acid,
        });
    }

    let candidate = choose_duration_candidate(duration_seconds);
    Ok(LoopWavAnalysis {
        duration_seconds,
        bpm: candidate.bpm,
        beats: candidate.beats,
        meter_numerator: 4,
        meter_denominator: 4,
        measures: candidate.measures,
        source: LoopAnalysisSource::DurationEstimate,
    })
}

fn valid_acid(acid: &AcidMetadata) -> bool {
    acid.flags & ACID_FLAG_ONE_SHOT == 0
        && acid.beats > 0
        && acid.meter_numerator > 0
        && acid.meter_denominator > 0
        && acid.tempo.is_finite()
        && acid.tempo > 0.0
}

fn duration_candidates(duration_seconds: f64) -> [TempoCandidate; 2] {
    [
        TempoCandidate {
            bpm: 240.0 / duration_seconds,
            beats: 4,
            measures: 1,
        },
        TempoCandidate {
            bpm: 480.0 / duration_seconds,
            beats: 8,
            measures: 2,
        },
    ]
}

fn choose_duration_candidate(duration_seconds: f64) -> TempoCandidate {
    let candidates = duration_candidates(duration_seconds);
    if (candidates[1].bpm - 120.0).abs() < (candidates[0].bpm - 120.0).abs() {
        candidates[1]
    } else {
        candidates[0]
    }
}

pub(crate) fn format_analysis(analysis: LoopWavAnalysis) -> String {
    let estimate = matches!(analysis.source, LoopAnalysisSource::DurationEstimate)
        .then_some("~")
        .unwrap_or("");
    let rounded = analysis.bpm.round();
    let bpm = if (analysis.bpm - rounded).abs() < 0.005 {
        format!("{rounded:.0}")
    } else {
        format!("{:.2}", analysis.bpm)
    };
    format!(
        "[{estimate}BPM{bpm} beat{} {}meas]",
        analysis.beats, analysis.measures
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn chunk(id: &[u8; 4], payload: &[u8]) -> Vec<u8> {
        let mut output = Vec::new();
        output.extend_from_slice(id);
        output.extend_from_slice(&(payload.len() as u32).to_le_bytes());
        output.extend_from_slice(payload);
        if !payload.len().is_multiple_of(2) {
            output.push(0);
        }
        output
    }

    fn wave(chunks: &[Vec<u8>]) -> Vec<u8> {
        let mut body = b"WAVE".to_vec();
        for chunk in chunks {
            body.extend_from_slice(chunk);
        }
        let mut output = b"RIFF".to_vec();
        output.extend_from_slice(&(body.len() as u32).to_le_bytes());
        output.extend_from_slice(&body);
        output
    }

    fn fmt_chunk(byte_rate: u32) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&1_u16.to_le_bytes());
        payload.extend_from_slice(&2_u16.to_le_bytes());
        payload.extend_from_slice(&44_100_u32.to_le_bytes());
        payload.extend_from_slice(&byte_rate.to_le_bytes());
        payload.extend_from_slice(&4_u16.to_le_bytes());
        payload.extend_from_slice(&16_u16.to_le_bytes());
        chunk(b"fmt ", &payload)
    }

    fn acid_chunk(flags: u32, beats: u32, numerator: u16, denominator: u16, bpm: f32) -> Vec<u8> {
        let mut payload = Vec::new();
        payload.extend_from_slice(&flags.to_le_bytes());
        payload.extend_from_slice(&49_u16.to_le_bytes());
        payload.extend_from_slice(&128_u16.to_le_bytes());
        payload.extend_from_slice(&0_f32.to_le_bytes());
        payload.extend_from_slice(&beats.to_le_bytes());
        payload.extend_from_slice(&denominator.to_le_bytes());
        payload.extend_from_slice(&numerator.to_le_bytes());
        payload.extend_from_slice(&bpm.to_le_bytes());
        chunk(b"acid", &payload)
    }

    #[test]
    fn reads_acid_after_data_and_honors_all_beat_counts() {
        for (beats, measures) in [(2, 1), (4, 1), (8, 2), (16, 4)] {
            let bytes = wave(&[
                fmt_chunk(176_400),
                chunk(b"JUNK", b"odd"),
                chunk(b"data", &vec![0; 176_400]),
                acid_chunk(0, beats, 4, 4, 99.353_23),
            ]);
            let analysis = analyze_reader(&mut Cursor::new(bytes)).unwrap();
            assert_eq!(analysis.source, LoopAnalysisSource::Acid);
            assert_eq!(analysis.beats, beats);
            assert_eq!(analysis.measures, measures);
            assert!((analysis.bpm - 99.353_23).abs() < 0.001);
        }
    }

    #[test]
    fn duration_fallback_lists_candidates_and_picks_nearest_to_120() {
        assert_eq!(duration_candidates(4.0)[0].bpm, 60.0);
        assert_eq!(duration_candidates(4.0)[1].bpm, 120.0);
        assert_eq!(choose_duration_candidate(4.0).measures, 2);
        assert_eq!(choose_duration_candidate(3.0).measures, 1);

        let bytes = wave(&[fmt_chunk(100), chunk(b"data", &vec![0; 400])]);
        let analysis = analyze_reader(&mut Cursor::new(bytes)).unwrap();
        assert_eq!(analysis.source, LoopAnalysisSource::DurationEstimate);
        assert_eq!(analysis.bpm, 120.0);
        assert_eq!(analysis.beats, 8);
        assert_eq!(format_analysis(analysis), "[~BPM120 beat8 2meas]");
    }

    #[test]
    fn ignores_one_shot_acid_and_rejects_out_of_bounds_chunks() {
        let bytes = wave(&[
            fmt_chunk(100),
            chunk(b"data", &vec![0; 400]),
            acid_chunk(ACID_FLAG_ONE_SHOT, 16, 4, 4, 200.0),
        ]);
        let analysis = analyze_reader(&mut Cursor::new(bytes)).unwrap();
        assert_eq!(analysis.source, LoopAnalysisSource::DurationEstimate);

        let mut corrupt = wave(&[fmt_chunk(100), chunk(b"data", &vec![0; 400])]);
        corrupt[40..44].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(analyze_reader(&mut Cursor::new(corrupt)).is_err());
    }
}
