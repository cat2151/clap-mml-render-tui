//! レンダリング結果を「数字 1 行」へ落とす。判定はすべてここを通す。
//!
//! 目で見ず耳でも聴かずに済ませるための層なので、**曖昧な指標を置かない**。
//! ここにあるのは「無音か」「同じ出音か」「どれだけ違うか」の 3 つだけ。

/// レンダリング 1 本ぶんの要約。
#[derive(Clone, Debug)]
pub(crate) struct RenderStats {
    /// ステレオ 1 組を 1 とした長さ。
    pub(crate) frames: usize,
    pub(crate) duration_ms: u64,
    pub(crate) peak: f32,
    pub(crate) rms: f64,
    /// 出音そのものの指紋。**音色を替えたのに一致したら差し替わっていない。**
    pub(crate) digest: u64,
    /// プラグインが名乗った音色名。どの音色が載ったかの裏取りに使う。
    pub(crate) patch_name: String,
    samples: Vec<f32>,
}

/// これを下回ったら無音とみなす。24bit の最下位ビット相当より十分小さい値。
const SILENCE_PEAK: f32 = 1e-6;

impl RenderStats {
    pub(crate) fn of(samples: &[f32], sample_rate: u32, patch_name: &str) -> Self {
        let frames = samples.len() / 2;
        let peak = samples.iter().fold(0.0_f32, |peak, s| peak.max(s.abs()));
        Self {
            frames,
            duration_ms: match sample_rate {
                0 => 0,
                rate => (frames as u64 * 1000) / rate as u64,
            },
            peak,
            rms: rms(samples),
            digest: digest(samples),
            patch_name: patch_name.to_string(),
            samples: samples.to_vec(),
        }
    }

    pub(crate) fn is_silent(&self) -> bool {
        self.peak < SILENCE_PEAK
    }
}

/// 二乗平均平方根。無音判定と、差の大きさを正規化する分母に使う。
pub(crate) fn rms(samples: &[f32]) -> f64 {
    if samples.is_empty() {
        return 0.0;
    }
    let sum: f64 = samples.iter().map(|s| *s as f64 * *s as f64).sum();
    (sum / samples.len() as f64).sqrt()
}

/// 出音そのものの指紋（FNV-1a 64）。
///
/// 浮動小数のビット列をそのまま混ぜる。**丸めを入れない**のは、
/// 「別の音色が載った」と「同じ音色が鳴り続けている」を区別するのが目的で、
/// 近いかどうかは [`diff_ratio`] の仕事だから。
pub(crate) fn digest(samples: &[f32]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for sample in samples {
        for byte in sample.to_bits().to_le_bytes() {
            hash ^= byte as u64;
            hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
        }
    }
    hash
}

/// 2 本の出音がどれだけ違うか。0 なら完全一致、1 前後で「まったく別物」。
///
/// 長さが違うぶんは短いほうを 0 で埋めて数える。**切り詰めると
/// 「和音のほうが長く鳴っている」ぶんが差から消える。**
pub(crate) fn diff_ratio(left: &RenderStats, right: &RenderStats) -> f64 {
    let len = left.samples.len().max(right.samples.len());
    if len == 0 {
        return 0.0;
    }
    let mut sum = 0.0_f64;
    for index in 0..len {
        let a = left.samples.get(index).copied().unwrap_or(0.0) as f64;
        let b = right.samples.get(index).copied().unwrap_or(0.0) as f64;
        sum += (a - b) * (a - b);
    }
    let diff = (sum / len as f64).sqrt();
    let scale = left.rms.max(right.rms);
    if scale <= 0.0 {
        // 両方とも無音。差も 0 なので「一致」を返す。
        return 0.0;
    }
    diff / scale
}

/// 出音が何通りあったか。音色を替えたぶんだけ増えるのが正しい。
pub(crate) fn distinct_digests(digests: &[u64]) -> usize {
    digests
        .iter()
        .collect::<std::collections::HashSet<_>>()
        .len()
}

/// 任意の文字列を、そのままファイル名にできる形へ潰す。
pub(crate) fn file_stem_for(value: &str) -> String {
    let cleaned: String = value
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let trimmed = cleaned.trim_matches('_');
    // 名前が長くなりすぎると Windows のパス長上限に当たる。
    let cut: String = trimmed.chars().take(48).collect();
    if cut.is_empty() {
        "render".to_string()
    } else {
        cut
    }
}

#[cfg(test)]
mod tests;
