//! live と offline の、行の頭の包絡のずれ。
//!
//! 頭が欠けると、検出した頭から先の包絡は offline の包絡を後ろへずらした形になる。
//! 両方の頭から同じ長さの包絡を取り、形の差が最小になるずれを測る。

/// 包絡の 1 点の長さ。
const ENVELOPE_BLOCK_MS: f64 = 1.0;

fn block_frames(sample_rate: u32) -> usize {
    ((ENVELOPE_BLOCK_MS * f64::from(sample_rate) / 1000.0).round() as usize).max(1)
}

/// インターリーブステレオの `samples` の `start` frame から `blocks` 点ぶんの、block ごとの peak。
fn envelope(samples: &[f32], sample_rate: u32, start: usize, blocks: usize) -> Vec<f32> {
    let block = block_frames(sample_rate);
    let frames = samples.len() / 2;
    (0..blocks)
        .map(|index| {
            let from = (start + index * block).min(frames);
            let to = (from + block).min(frames);
            samples[from * 2..to * 2]
                .iter()
                .fold(0.0_f32, |peak, sample| peak.max(sample.abs()))
        })
        .collect()
}

/// `live` の `live_start` からの包絡を、`reference` の `reference_start` を中心にずらした包絡と
/// 比べ、差（各包絡をその peak で割ったものの二乗平均）が最小になるずれ（ms。正 = live が
/// 遅れている、負 = live の頭が欠けている）を返す。比べる長さは `window_ms`、探すずれは
/// ±`max_lag_ms`。どちらかが無音なら `None`。
///
/// 立ち上がりの無い（指数減衰だけの）包絡は、ずらしても peak で割ると同じ形になるので、
/// 頭の欠けはずれに出ない（クリックとして出る）。
pub(super) fn envelope_lag_ms(
    live: &[f32],
    live_start: usize,
    reference: &[f32],
    reference_start: usize,
    sample_rate: u32,
    window_ms: f64,
    max_lag_ms: f64,
) -> Option<f64> {
    let block = block_frames(sample_rate);
    let window = (window_ms / ENVELOPE_BLOCK_MS).round() as usize;
    let max_lag = (max_lag_ms / ENVELOPE_BLOCK_MS).round() as usize;
    let live = normalized(envelope(live, sample_rate, live_start, window))?;
    let reference_peak = peak(&envelope(reference, sample_rate, reference_start, window));
    if reference_peak == 0.0 {
        return None;
    }
    // reference は live の前後へずらすので、探す幅ぶん前から取る（頭より前は 0 とみなす）。
    let before = (reference_start / block).min(max_lag);
    let reference = envelope(
        reference,
        sample_rate,
        reference_start - before * block,
        before + window + max_lag,
    );
    let reference_at = |index: isize| -> f32 {
        usize::try_from(index + before as isize)
            .ok()
            .and_then(|at| reference.get(at))
            .map_or(0.0, |value| value / reference_peak)
    };

    let max_lag = max_lag as isize;
    (-max_lag..=max_lag)
        .map(|lag| {
            let error = live
                .iter()
                .enumerate()
                .map(|(index, &value)| (value - reference_at(index as isize - lag)).powi(2))
                .sum::<f32>();
            (error, lag)
        })
        .min_by(|(left, _), (right, _)| left.total_cmp(right))
        .map(|(_, lag)| lag as f64 * ENVELOPE_BLOCK_MS)
}

fn peak(values: &[f32]) -> f32 {
    values.iter().fold(0.0_f32, |peak, value| peak.max(*value))
}

fn normalized(values: Vec<f32>) -> Option<Vec<f32>> {
    let peak = peak(&values);
    (peak > 0.0).then(|| values.into_iter().map(|value| value / peak).collect())
}

#[cfg(test)]
mod tests;
