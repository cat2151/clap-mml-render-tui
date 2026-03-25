/// 小節の再生サンプルがどこから解決されたかを表す。
#[derive(Clone)]
pub(in crate::daw::playback) enum PlaybackMeasureSource {
    Empty,
    Cache { tracks: Vec<usize> },
    Render,
}

impl PlaybackMeasureSource {
    pub(in crate::daw::playback) fn build_log_line(&self, measure_number: usize) -> String {
        match self {
            Self::Empty => format!("play: start meas{measure_number} empty -> silence"),
            Self::Cache { tracks } => {
                if tracks.is_empty() {
                    format!("play: start meas{measure_number} cache empty-tracks")
                } else {
                    let cache_entries = tracks
                        .iter()
                        .map(|track| format!("track{track}/meas{measure_number}"))
                        .collect::<Vec<_>>()
                        .join(", ");
                    format!("play: start meas{measure_number} cache {cache_entries}")
                }
            }
            Self::Render => format!("play: start meas{measure_number} render fallback"),
        }
    }
}

/// 1 小節ぶんの再生素材と、その解決元をまとめた値。
pub(in crate::daw::playback) struct PlaybackMeasureAudio {
    pub(in crate::daw::playback) samples: Vec<f32>,
    pub(in crate::daw::playback) source: PlaybackMeasureSource,
}

/// すでに鳴り始めた小節の full size サンプルと、その消費位置を保持する。
///
/// `offset` はこのレイヤーからすでに再生済みのインターリーブサンプル数で、
/// 次の小節チャンクを組み立てるときに未再生の余韻だけを重ねるために使う。
pub(in crate::daw::playback) struct ActiveMeasureLayer {
    samples: Vec<f32>,
    offset: usize,
}

/// 現在の小節チャンクを生成し、前小節までの余韻と新しい小節の先頭を重ねて返す。
///
/// `new_measure_samples` には今この小節境界で鳴り始める full size サンプル全体を渡す。
/// `measure_samples` は 1 小節ぶんの標準サンプル数で、返り値も常にこの長さになる。
/// 既存レイヤーの余韻は未再生区間だけが加算され、再生し終えたレイヤーは除去される。
pub(in crate::daw::playback) fn mix_measure_chunk(
    active_layers: &mut Vec<ActiveMeasureLayer>,
    new_measure_samples: Vec<f32>,
    measure_samples: usize,
) -> Vec<f32> {
    active_layers.push(ActiveMeasureLayer {
        samples: new_measure_samples,
        offset: 0,
    });

    let mut mixed = vec![0.0f32; measure_samples];
    for layer in active_layers.iter_mut() {
        let remaining = &layer.samples[layer.offset..];
        let chunk_len = remaining.len().min(measure_samples);
        for i in 0..chunk_len {
            mixed[i] += remaining[i];
        }
        layer.offset += chunk_len;
    }
    active_layers.retain(|layer| layer.offset < layer.samples.len());

    mixed
}
