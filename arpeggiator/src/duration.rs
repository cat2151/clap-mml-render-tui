//! 希望 duration を、同じ声部が次に鳴る step で打ち切る。
//!
//! 同じ声部は同じ音高なので、前の音が鳴り終わる前に次が来ると retrigger で潰れる。
//! 「先に鳴っていたほうを短くする」というシンプルな解決に統一し、鳴らす位置は
//! 動かさない。別の声部との重なりは和音として成立するので打ち切らない。

/// `voices[i]` を step `i` で鳴らすときの、各音の実 duration（step 数）。
///
/// - `desired[i]` は候補から引いた希望長。足りない分は 1 step 扱い。
/// - 同じ声部が次に現れる step で打ち切る。
/// - 小節末（`voices.len()`）は越えない。
/// - 返す値は必ず 1 以上。
pub fn clamp_durations(voices: &[usize], desired: &[usize]) -> Vec<usize> {
    voices
        .iter()
        .enumerate()
        .map(|(step, voice)| {
            let limit = voices[step + 1..]
                .iter()
                .position(|next| next == voice)
                .map_or(voices.len() - step, |offset| offset + 1);
            desired.get(step).copied().unwrap_or(1).clamp(1, limit)
        })
        .collect()
}

#[cfg(test)]
mod tests;
