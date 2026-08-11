//! アルペジエータ。声部の並び（音型）と各音の長さを組み立てる。
//!
//! 1 step につき必ず1音を置き、duration だけを候補から引く。音高は持たない。

use rand::RngExt;

mod duration;
mod pattern;

pub use duration::clamp_durations;
pub use pattern::ArpPattern;

/// duration の候補（step 数）。1 step を軸に、2/4 step の伸びを混ぜる。
pub const ARP_DURATION_CHOICES: [usize; 3] = [1, 2, 4];

/// アルペジオ1音。`voice` は 0 が最低声部。
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ArpNote {
    pub step: usize,
    pub voice: usize,
    pub duration_steps: usize,
}

/// `pattern` を `steps` step ぶん展開する。1 step につき必ず1音を置く。
///
/// duration は [`ARP_DURATION_CHOICES`] から引いたうえで、同じ声部が次に来る step と
/// 小節末で打ち切る（[`clamp_durations`]）。`voice_count` か `steps` が 0 なら空。
pub fn generate_arpeggio(
    pattern: ArpPattern,
    voice_count: usize,
    steps: usize,
    rng: &mut impl RngExt,
) -> Vec<ArpNote> {
    if voice_count == 0 || steps == 0 {
        return Vec::new();
    }
    let voices = voice_series(pattern, voice_count, steps, rng);
    let desired = (0..steps)
        .map(|_| ARP_DURATION_CHOICES[rng.random_range(0..ARP_DURATION_CHOICES.len())])
        .collect::<Vec<_>>();
    let durations = clamp_durations(&voices, &desired);
    voices
        .into_iter()
        .zip(durations)
        .enumerate()
        .map(|(step, (voice, duration_steps))| ArpNote {
            step,
            voice,
            duration_steps,
        })
        .collect()
}

/// 音型の周期を `steps` ぶん繰り返す。周期を持たない `Random` だけ毎 step 引き直す。
fn voice_series(
    pattern: ArpPattern,
    voice_count: usize,
    steps: usize,
    rng: &mut impl RngExt,
) -> Vec<usize> {
    match pattern.voice_sequence(voice_count) {
        Some(period) if !period.is_empty() => {
            (0..steps).map(|step| period[step % period.len()]).collect()
        }
        // 周期が空になるのは voice_count == 0 のときだけ。呼び出し元で弾いてある。
        Some(_) | None => (0..steps)
            .map(|_| rng.random_range(0..voice_count))
            .collect(),
    }
}

#[cfg(test)]
mod tests;
