//! 行ごとに、ランプを張る区間を発音表から決める。
//!
//! 区間を小節まるごとに固定すると、音が中央に2つしかない行では2値の間の
//! 狭い範囲しか使われず起伏が出ない。実際に鳴る範囲へ張り直して、必ず端の値が
//! 出るようにする。
//!
//! CC1 と velocity で終点が違うのは、値を使うタイミングが違うから。velocity は
//! note on の瞬間にしか載らないので最後の note on で終え、CC1 は音が伸びている
//! 間も効くので最後の音が鳴り終わるところまで伸ばす。

use super::pattern::RampSpan;
use crate::GRID_STEPS;

/// velocity のランプ区間。最初の note on から最後の note on まで。
///
/// 音が1つしかない行は幅が潰れ、その音は始点の値で鳴る。
pub(super) fn velocity_span(triggers: &[bool; GRID_STEPS]) -> RampSpan {
    match sounding_range(triggers) {
        Some((first, last)) => RampSpan {
            start: first,
            end: last,
        },
        None => RampSpan::WHOLE_MEASURE,
    }
}

/// CC1 のランプ区間。最初の note on から、最後の音が鳴り終わる step まで。
///
/// 終点は小節末尾でクランプする。chord mode の和音行（step 0 に全音符）は
/// これで「小節頭が始点、小節末尾が終点」になる。
pub(super) fn cc1_span(triggers: &[bool; GRID_STEPS], sounding_end: Option<usize>) -> RampSpan {
    match sounding_range(triggers) {
        Some((first, _)) => RampSpan {
            start: first,
            end: sounding_end.unwrap_or(first).min(GRID_STEPS - 1),
        },
        None => RampSpan::WHOLE_MEASURE,
    }
}

/// 最初と最後の note on の step。1つも鳴らない行は `None`。
fn sounding_range(triggers: &[bool; GRID_STEPS]) -> Option<(usize, usize)> {
    let first = triggers.iter().position(|trigger| *trigger)?;
    let last = triggers
        .iter()
        .rposition(|trigger| *trigger)
        .expect("a first trigger was just found");
    Some((first, last))
}

#[cfg(test)]
mod tests;
