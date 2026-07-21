use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

pub(super) struct TrackSink {
    pub(super) track: usize,
    pub(super) sink: Arc<rodio::Sink>,
}

#[derive(Clone, Copy, Debug, Default)]
pub(super) struct PlayPathTiming {
    pub(super) open: Duration,
    pub(super) decode: Duration,
    pub(super) sink: Duration,
    pub(super) append: Duration,
}

pub(super) fn play_path(
    handle: &rodio::OutputStreamHandle,
    path: &Path,
) -> Result<Arc<rodio::Sink>> {
    play_path_profiled(handle, path).0
}

pub(super) fn play_path_profiled(
    handle: &rodio::OutputStreamHandle,
    path: &Path,
) -> (Result<Arc<rodio::Sink>>, PlayPathTiming) {
    let mut timing = PlayPathTiming::default();

    let stage = Instant::now();
    let file = match File::open(path)
        .with_context(|| format!("WAVを開けません: {}", path.display()))
    {
        Ok(file) => file,
        Err(error) => {
            timing.open = stage.elapsed();
            return (Err(error), timing);
        }
    };
    timing.open = stage.elapsed();

    let stage = Instant::now();
    let source = match rodio::Decoder::new(BufReader::new(file))
        .with_context(|| format!("WAVをdecodeできません: {}", path.display()))
    {
        Ok(source) => source,
        Err(error) => {
            timing.decode = stage.elapsed();
            return (Err(error), timing);
        }
    };
    timing.decode = stage.elapsed();

    let stage = Instant::now();
    let sink = match rodio::Sink::try_new(handle) {
        Ok(sink) => Arc::new(sink),
        Err(error) => {
            timing.sink = stage.elapsed();
            return (Err(error.into()), timing);
        }
    };
    timing.sink = stage.elapsed();

    let stage = Instant::now();
    sink.append(source);
    timing.append = stage.elapsed();
    (Ok(sink), timing)
}

pub(super) fn stop_sinks(sinks: &mut Vec<TrackSink>) {
    for voice in sinks.drain(..) {
        voice.sink.stop();
    }
}

pub(super) fn stop_pad_sinks(sinks: &mut HashMap<char, Arc<rodio::Sink>>) {
    for (_, sink) in sinks.drain() {
        sink.stop();
    }
}

pub(super) fn take_pad_voice<T>(voices: &mut HashMap<char, T>, pad: char) -> Option<T> {
    voices.remove(&pad)
}
