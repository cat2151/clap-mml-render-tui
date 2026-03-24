//! DAW キャッシュ用 WAV 入出力

use std::path::Path;

pub(super) struct WavCacheInfo {
    pub(super) spec: hound::WavSpec,
    pub(super) interleaved_sample_count: usize,
}

pub(super) fn read_wav_cache_info(path: &Path) -> anyhow::Result<WavCacheInfo> {
    let reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    let interleaved_sample_count = reader.duration() as usize * spec.channels as usize;
    Ok(WavCacheInfo {
        spec,
        interleaved_sample_count,
    })
}

pub(super) fn load_wav_samples(path: &Path) -> anyhow::Result<Vec<f32>> {
    let mut reader = hound::WavReader::open(path)?;
    let spec = reader.spec();
    match spec.sample_format {
        hound::SampleFormat::Float => Ok(reader.samples::<f32>().collect::<Result<Vec<_>, _>>()?),
        hound::SampleFormat::Int => {
            let scale = ((1_i64 << (spec.bits_per_sample.saturating_sub(1) as u32)) - 1) as f32;
            let samples = reader
                .samples::<i32>()
                .map(|sample| sample.map(|value| value as f32 / scale))
                .collect::<Result<Vec<_>, _>>()?;
            Ok(samples)
        }
    }
}
