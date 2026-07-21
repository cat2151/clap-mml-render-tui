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
pub enum LoopAnalysisSource {
    Acid,
    DurationEstimate,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LoopWavKind {
    #[default]
    Loop,
    OneShot,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct LoopTempoAnalysis {
    /// WAV の実長・beat 数・拍子から算出した、再生に使用する実効 BPM。
    pub bpm: f64,
    /// ACID chunk に宣言されていた BPM。実効 BPM との不整合調査用に保持する。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub declared_bpm: Option<f64>,
    pub beats: u32,
    pub meter_numerator: u16,
    pub meter_denominator: u16,
    pub source: LoopAnalysisSource,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct LoopWavAnalysis {
    pub duration_seconds: f64,
    #[serde(default)]
    pub kind: LoopWavKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tempo: Option<LoopTempoAnalysis>,
    /// grid・波形表示で占有する小節数。one-shot は常に1。
    pub measures: usize,
}

impl LoopWavAnalysis {
    pub fn into_one_shot(self) -> Self {
        Self {
            duration_seconds: self.duration_seconds,
            kind: LoopWavKind::OneShot,
            tempo: None,
            measures: 1,
        }
    }
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

pub fn analyze_file(path: &Path) -> Result<LoopWavAnalysis> {
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

    if acid.is_some_and(|acid| acid.flags & ACID_FLAG_ONE_SHOT != 0) {
        return Ok(LoopWavAnalysis {
            duration_seconds,
            kind: LoopWavKind::OneShot,
            tempo: None,
            measures: 1,
        });
    }

    if let Some(acid) = acid.filter(valid_acid_loop) {
        let measures = (acid.beats as usize)
            .div_ceil(usize::from(acid.meter_numerator))
            .max(1);
        let bpm = effective_bpm(acid.beats, acid.meter_denominator, duration_seconds)?;
        return Ok(LoopWavAnalysis {
            duration_seconds,
            kind: LoopWavKind::Loop,
            tempo: Some(LoopTempoAnalysis {
                bpm,
                declared_bpm: Some(f64::from(acid.tempo)),
                beats: acid.beats,
                meter_numerator: acid.meter_numerator,
                meter_denominator: acid.meter_denominator,
                source: LoopAnalysisSource::Acid,
            }),
            measures,
        });
    }

    let candidate = choose_duration_candidate(duration_seconds);
    Ok(LoopWavAnalysis {
        duration_seconds,
        kind: LoopWavKind::Loop,
        tempo: Some(LoopTempoAnalysis {
            bpm: candidate.bpm,
            declared_bpm: None,
            beats: candidate.beats,
            meter_numerator: 4,
            meter_denominator: 4,
            source: LoopAnalysisSource::DurationEstimate,
        }),
        measures: candidate.measures,
    })
}

fn effective_bpm(beats: u32, meter_denominator: u16, duration_seconds: f64) -> Result<f64> {
    let bpm = f64::from(beats) * 60.0 * 4.0 / f64::from(meter_denominator) / duration_seconds;
    if !bpm.is_finite() || bpm <= 0.0 {
        anyhow::bail!("ACID beat数とWAV実長からBPMを算出できません");
    }
    Ok(bpm)
}

fn valid_acid_loop(acid: &AcidMetadata) -> bool {
    acid.beats > 0
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

pub fn format_analysis(analysis: LoopWavAnalysis) -> String {
    if analysis.kind == LoopWavKind::OneShot {
        return format!("[one-shot {}]", format_duration(analysis.duration_seconds));
    }
    let Some(tempo) = analysis.tempo else {
        return "[loop metadata missing]".to_string();
    };
    let estimate = matches!(tempo.source, LoopAnalysisSource::DurationEstimate)
        .then_some("~")
        .unwrap_or("");
    let rounded = tempo.bpm.round();
    let bpm = if (tempo.bpm - rounded).abs() < 0.005 {
        format!("{rounded:.0}")
    } else {
        format!("{:.2}", tempo.bpm)
    };
    let declared = tempo
        .declared_bpm
        .filter(|declared| (declared - tempo.bpm).abs() >= 0.5)
        .map(|declared| format!(" (ACID{})", format_bpm_value(declared)))
        .unwrap_or_default();
    format!(
        "[{estimate}BPM{bpm}{declared} beat{} {}meas]",
        tempo.beats, analysis.measures
    )
}

fn format_duration(seconds: f64) -> String {
    if seconds < 10.0 {
        format!("{seconds:.2}s")
    } else {
        format!("{seconds:.1}s")
    }
}

fn format_bpm_value(bpm: f64) -> String {
    let rounded = bpm.round();
    if (bpm - rounded).abs() < 0.005 {
        format!("{rounded:.0}")
    } else {
        format!("{bpm:.2}")
    }
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
    fn reads_acid_after_data_and_uses_duration_for_effective_bpm() {
        for (beats, measures) in [(2, 1), (4, 1), (8, 2), (16, 4)] {
            let bytes = wave(&[
                fmt_chunk(176_400),
                chunk(b"JUNK", b"odd"),
                chunk(b"data", &vec![0; 176_400]),
                acid_chunk(0, beats, 4, 4, 99.353_23),
            ]);
            let analysis = analyze_reader(&mut Cursor::new(bytes)).unwrap();
            let tempo = analysis.tempo.unwrap();
            assert_eq!(analysis.kind, LoopWavKind::Loop);
            assert_eq!(tempo.source, LoopAnalysisSource::Acid);
            assert_eq!(tempo.beats, beats);
            assert_eq!(analysis.measures, measures);
            assert_eq!(tempo.bpm, f64::from(beats) * 60.0);
            assert_eq!(tempo.declared_bpm, Some(99.353_23_f32 as f64));
        }
    }

    #[test]
    fn acid_effective_bpm_accounts_for_meter_denominator() {
        let bytes = wave(&[
            fmt_chunk(100),
            chunk(b"data", &vec![0; 400]),
            acid_chunk(0, 8, 4, 8, 120.0),
        ]);

        let analysis = analyze_reader(&mut Cursor::new(bytes)).unwrap();

        assert_eq!(analysis.tempo.unwrap().bpm, 60.0);
        assert_eq!(analysis.tempo.unwrap().declared_bpm, Some(120.0));
        assert_eq!(format_analysis(analysis), "[BPM60 (ACID120) beat8 2meas]");
    }

    #[test]
    fn duration_fallback_lists_candidates_and_picks_nearest_to_120() {
        assert_eq!(duration_candidates(4.0)[0].bpm, 60.0);
        assert_eq!(duration_candidates(4.0)[1].bpm, 120.0);
        assert_eq!(choose_duration_candidate(4.0).measures, 2);
        assert_eq!(choose_duration_candidate(3.0).measures, 1);

        let bytes = wave(&[fmt_chunk(100), chunk(b"data", &vec![0; 400])]);
        let analysis = analyze_reader(&mut Cursor::new(bytes)).unwrap();
        let tempo = analysis.tempo.unwrap();
        assert_eq!(tempo.source, LoopAnalysisSource::DurationEstimate);
        assert_eq!(tempo.bpm, 120.0);
        assert_eq!(tempo.declared_bpm, None);
        assert_eq!(tempo.beats, 8);
        assert_eq!(format_analysis(analysis), "[~BPM120 beat8 2meas]");
    }

    #[test]
    fn acid_one_shot_after_data_has_no_tempo_and_rejects_out_of_bounds_chunks() {
        let bytes = wave(&[
            fmt_chunk(100),
            chunk(b"data", &vec![0; 400]),
            acid_chunk(ACID_FLAG_ONE_SHOT, 16, 4, 4, 200.0),
        ]);
        let analysis = analyze_reader(&mut Cursor::new(bytes)).unwrap();
        assert_eq!(analysis.kind, LoopWavKind::OneShot);
        assert_eq!(analysis.tempo, None);
        assert_eq!(analysis.measures, 1);
        assert_eq!(format_analysis(analysis), "[one-shot 4.00s]");

        let mut corrupt = wave(&[fmt_chunk(100), chunk(b"data", &vec![0; 400])]);
        corrupt[40..44].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(analyze_reader(&mut Cursor::new(corrupt)).is_err());
    }
}
