use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

use anyhow::{Context, Result};

pub(super) struct TrackSink {
    pub(super) track: usize,
    pub(super) sink: Arc<rodio::Sink>,
}

pub(super) fn play_path(
    handle: &rodio::OutputStreamHandle,
    path: &Path,
) -> Result<Arc<rodio::Sink>> {
    let file = File::open(path).with_context(|| format!("WAVを開けません: {}", path.display()))?;
    let source = rodio::Decoder::new(BufReader::new(file))
        .with_context(|| format!("WAVをdecodeできません: {}", path.display()))?;
    let sink = Arc::new(rodio::Sink::try_new(handle)?);
    sink.append(source);
    Ok(sink)
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
