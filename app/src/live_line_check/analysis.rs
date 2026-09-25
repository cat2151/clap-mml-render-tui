//! 録れたステレオ波形（インターリーブ）から、クリック候補・音の途中の無音・各行の頭を数える。

/// クリック候補とみなす倍率の既定値。周囲の差の中央値のこの倍を超えた点を拾う。
pub(crate) const DEFAULT_CLICK_K: f32 = 8.0;

/// これ未満のサンプル間の差はクリックとみなさない。無音の中の微小な揺れを拾わないため。
const CLICK_FLOOR: f32 = 0.02;

/// 中央値を取る周囲の幅。
const CLICK_WINDOW_MS: f64 = 10.0;

/// これより近いクリック候補は 1 つにまとめる。
const CLICK_MERGE_MS: f64 = 1.0;

/// これ以上続く完全な 0 を「音の途中の無音」の候補にする。
const DROPOUT_MIN_MS: f64 = 5.0;

/// 無音の前後がこの peak 以上なら「鳴っている区間に挟まった」とみなす。
const SOUNDING_PEAK: f32 = 1.0e-3;

/// 頭の検出に使う block の長さ。
const ONSET_BLOCK_MS: f64 = 1.0;

/// 頭とみなす block peak の下限。
const ONSET_FLOOR: f32 = 1.0e-3;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Click {
    pub(crate) frame: usize,
    /// サンプル間の差（左右の大きい方）。
    pub(crate) magnitude: f32,
    /// 差が周囲の中央値の何倍か。中央値が 0 なら無限大。
    pub(crate) ratio: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Dropout {
    pub(crate) frame: usize,
    pub(crate) frames: usize,
}

fn frame_count(samples: &[f32]) -> usize {
    samples.len() / 2
}

fn frame_peak(samples: &[f32], frame: usize) -> f32 {
    samples[frame * 2].abs().max(samples[frame * 2 + 1].abs())
}

fn ms_to_frames(ms: f64, sample_rate: u32) -> usize {
    ((ms * f64::from(sample_rate)) / 1000.0).round().max(1.0) as usize
}

/// サンプル間の差が、周囲 10 ms の差の中央値の `k` 倍を超える点。
pub(crate) fn detect_clicks(samples: &[f32], sample_rate: u32, k: f32) -> Vec<Click> {
    let frames = frame_count(samples);
    if frames < 2 {
        return Vec::new();
    }
    let diffs: Vec<f32> = (0..frames)
        .map(|n| {
            if n == 0 {
                return 0.0;
            }
            let left = (samples[n * 2] - samples[(n - 1) * 2]).abs();
            let right = (samples[n * 2 + 1] - samples[(n - 1) * 2 + 1]).abs();
            left.max(right)
        })
        .collect();

    // 10 ms の窓を 5 ms ずつずらし、窓の中央 5 ms の点を窓全体の中央値と比べる。
    let window = ms_to_frames(CLICK_WINDOW_MS, sample_rate).max(2);
    let hop = window / 2;
    let merge = ms_to_frames(CLICK_MERGE_MS, sample_rate);
    let mut scratch = Vec::with_capacity(window + hop);
    let mut clicks: Vec<Click> = Vec::new();
    let mut hop_start = 0;
    while hop_start < frames {
        let hop_end = (hop_start + hop).min(frames);
        let from = hop_start.saturating_sub(hop / 2);
        let to = (hop_end + hop / 2).min(frames);
        scratch.clear();
        scratch.extend_from_slice(&diffs[from..to]);
        let median = median(&mut scratch);
        let threshold = (median * k).max(CLICK_FLOOR);
        for (frame, &magnitude) in diffs.iter().enumerate().take(hop_end).skip(hop_start) {
            if magnitude <= threshold {
                continue;
            }
            let ratio = if median > 0.0 {
                magnitude / median
            } else {
                f32::INFINITY
            };
            let click = Click {
                frame,
                magnitude,
                ratio,
            };
            match clicks.last_mut() {
                Some(last) if frame - last.frame < merge => {
                    if magnitude > last.magnitude {
                        *last = click;
                    }
                }
                _ => clicks.push(click),
            }
        }
        hop_start = hop_end;
    }
    clicks
}

fn median(values: &mut [f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mid = values.len() / 2;
    let (_, value, _) = values.select_nth_unstable_by(mid, f32::total_cmp);
    *value
}

/// 鳴っている区間に挟まった、5 ms 以上の完全な 0 の連続。
pub(crate) fn detect_dropouts(samples: &[f32], sample_rate: u32) -> Vec<Dropout> {
    let frames = frame_count(samples);
    let min_frames = ms_to_frames(DROPOUT_MIN_MS, sample_rate);
    let side = min_frames;
    let is_zero = |frame: usize| samples[frame * 2] == 0.0 && samples[frame * 2 + 1] == 0.0;
    let sounding = |from: usize, to: usize| {
        (from..to).any(|frame| frame_peak(samples, frame) >= SOUNDING_PEAK)
    };
    let mut dropouts = Vec::new();
    let mut frame = 0;
    while frame < frames {
        if !is_zero(frame) {
            frame += 1;
            continue;
        }
        let start = frame;
        while frame < frames && is_zero(frame) {
            frame += 1;
        }
        let end = frame;
        if end - start < min_frames || start == 0 || end == frames {
            continue;
        }
        if sounding(start.saturating_sub(side), start) && sounding(end, (end + side).min(frames)) {
            dropouts.push(Dropout {
                frame: start,
                frames: end - start,
            });
        }
    }
    dropouts
}

/// 各行の頭（送信した frame から、音が立ち上がる block の先頭まで）。
///
/// 送信直前から 1 ms block の peak を順に見て、そこまでの最小値の 2 倍（下限
/// [`ONSET_FLOOR`]）を初めて超えた block を頭とする。前の行の余韻が残っていても、
/// 切り替えで下がってから立ち上がる点を拾える。次の行の送信までに見つからなければ `None`。
pub(crate) fn line_heads(samples: &[f32], sample_rate: u32, sends: &[usize]) -> Vec<Option<usize>> {
    let frames = frame_count(samples);
    let block = ms_to_frames(ONSET_BLOCK_MS, sample_rate);
    sends
        .iter()
        .enumerate()
        .map(|(index, &send)| {
            let limit = sends.get(index + 1).copied().unwrap_or(frames).min(frames);
            let block_peak = |from: usize, to: usize| {
                (from..to)
                    .map(|frame| frame_peak(samples, frame))
                    .fold(0.0_f32, f32::max)
            };
            // 送信直前の block から数え始める。送信の瞬間に鳴り始める行も拾うため。
            let mut floor = block_peak(send.saturating_sub(block), send.min(frames));
            let mut start = send;
            while start < limit {
                let end = (start + block).min(limit);
                let peak = block_peak(start, end);
                if peak >= ONSET_FLOOR && peak > floor * 2.0 {
                    return Some(start - send);
                }
                floor = floor.min(peak);
                start = end;
            }
            None
        })
        .collect()
}

#[cfg(test)]
mod tests;
