use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};

pub struct TrackSink {
    pub track: usize,
    pub sink: Arc<rodio::Player>,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct PlayPathTiming {
    pub open: Duration,
    pub decode: Duration,
    pub sink: Duration,
    pub append: Duration,
}

pub fn play_path(handle: &rodio::mixer::Mixer, path: &Path) -> Result<Arc<rodio::Player>> {
    play_path_profiled(handle, path).0
}

pub fn play_path_profiled(
    handle: &rodio::mixer::Mixer,
    path: &Path,
) -> (Result<Arc<rodio::Player>>, PlayPathTiming) {
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
    let sink = Arc::new(rodio::Player::connect_new(handle));
    timing.sink = stage.elapsed();

    let stage = Instant::now();
    sink.append(source);
    timing.append = stage.elapsed();
    (Ok(sink), timing)
}

pub fn stop_sinks(sinks: &mut Vec<TrackSink>) {
    for voice in sinks.drain(..) {
        voice.sink.stop();
    }
}

pub fn stop_pad_sinks(sinks: &mut HashMap<char, Arc<rodio::Player>>) {
    for (_, sink) in sinks.drain() {
        sink.stop();
    }
}

pub fn take_pad_voice<T>(voices: &mut HashMap<char, T>, pad: char) -> Option<T> {
    voices.remove(&pad)
}
