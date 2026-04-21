use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{Arc, Mutex},
};

use super::wav_io::load_wav_samples;
use super::{
    batch_logging::{TrackRerenderBatch, TrackRerenderBatchCompletionContext},
    CacheJob, DawApp,
};

#[test]
fn load_wav_samples_reads_back_float_wav_cache() {
    let tmp = std::env::temp_dir().join(format!(
        "cmrt_test_daw_cache_{}_{}.wav",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate: 44_100,
        bits_per_sample: 32,
        sample_format: hound::SampleFormat::Float,
    };
    {
        let mut writer = hound::WavWriter::create(&tmp, spec).unwrap();
        writer.write_sample(0.25f32).unwrap();
        writer.write_sample(-0.25f32).unwrap();
        writer.finalize().unwrap();
    }

    let samples = load_wav_samples(&tmp).unwrap();
    std::fs::remove_file(&tmp).ok();

    assert_eq!(samples, vec![0.25, -0.25]);
}

#[path = "cache_rerender.rs"]
mod cache_rerender;
